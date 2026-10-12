#!/usr/bin/env python3
"""Updater fault and immutable per-user installation tests; no production state or network."""
import contextlib
import importlib.util
import io
import json
import os
from pathlib import Path
import runpy
import shutil
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[3]
spec = importlib.util.spec_from_file_location('ruwac_updates', ROOT / 'scripts/update-user.py')
updates = importlib.util.module_from_spec(spec)
spec.loader.exec_module(updates)
COMMIT = 'a' * 40


class Updates(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        self.environment = patch.dict(os.environ, {'XDG_CONFIG_HOME': str(self.root / 'config'), 'XDG_CACHE_HOME': str(self.root / 'cache')})
        self.environment.start()

    def tearDown(self):
        self.environment.stop()
        self.temp.cleanup()

    def metadata(self, **changes):
        release = {'tag_name': 'v0.2.1', 'draft': False, 'prerelease': False,
                   'html_url': 'https://github.com/w3fans/RUWAC/releases/tag/v0.2.1'}
        release.update(changes)
        return release

    def test_numeric_order_not_lexical_and_no_downgrade(self):
        data = self.metadata(tag_name='v0.2.10', html_url='https://github.com/w3fans/RUWAC/releases/tag/v0.2.10')
        with patch.object(updates, 'api', side_effect=[data, {'object': {'type': 'commit', 'sha': COMMIT}}]):
            self.assertTrue(updates.latest('0.2.9')['available'])
        with patch.object(updates, 'api', side_effect=[self.metadata(), {'object': {'type': 'commit', 'sha': COMMIT}}]):
            self.assertFalse(updates.latest('0.3.0')['available'])

    def test_reject_drafts_prereleases_wrong_origin_and_invalid_tag(self):
        for values in [{'draft': True}, {'prerelease': True}, {'html_url': 'https://example.org/'}, {'tag_name': 'v0.2.1;bad'}, {'tag_name': 'v0.02.1'}]:
            with self.subTest(values=values), patch.object(updates, 'api', return_value=self.metadata(**values)):
                with self.assertRaises(ValueError): updates.latest('0.2.0')

    def test_annotated_tag_resolves_exact_commit(self):
        with patch.object(updates, 'api', side_effect=[self.metadata(), {'object': {'type': 'tag', 'sha': 'b' * 40}}, {'object': {'type': 'commit', 'sha': COMMIT}}]) as api:
            self.assertEqual(updates.latest('0.2.0')['commit'], COMMIT)
            self.assertEqual(api.call_args.args[0], '/git/tags/' + 'b' * 40)

    def test_tag_changed_during_fetch_never_checks_out(self):
        with patch.object(updates, 'run') as run, patch.object(subprocess, 'check_output', return_value='b' * 40):
            with self.assertRaisesRegex(ValueError, 'changed'):
                updates.checkout({'tag': 'v0.2.1', 'commit': COMMIT}, self.root / 'source')
            self.assertFalse(any('checkout' in call.args[0] for call in run.call_args_list))

    def test_preferences_opt_out_preserves_unknown_and_symlink_files(self):
        self.assertTrue(updates.automatic_setting())
        self.assertFalse(updates.automatic_setting(False))
        self.assertFalse(updates.automatic_setting())
        path = updates.settings_path()
        path.write_text('{"schema":2,"automatic_check":true}')
        before = path.read_bytes()
        with self.assertRaises(ValueError): updates.automatic_setting(True)
        self.assertEqual(path.read_bytes(), before)
        path.unlink(); other = self.root / 'foreign'; other.write_text('preserve')
        path.symlink_to(other)
        with self.assertRaises(ValueError): updates.automatic_setting(True)
        self.assertEqual(other.read_text(), 'preserve')

    def test_disabled_automatic_check_performs_no_network(self):
        updates.automatic_setting(False)
        with patch.object(sys, 'argv', ['updater', '--current', '0.2.0', '--check-auto']), patch.object(updates, 'api') as api, contextlib.redirect_stdout(io.StringIO()) as output:
            updates.main()
        api.assert_not_called()
        self.assertFalse(json.loads(output.getvalue())['available'])

    def test_lock_contention_and_unsafe_lock(self):
        path = self.root / 'lock'; held = updates.locked_file(path)
        try:
            with self.assertRaisesRegex(ValueError, 'active'): updates.locked_file(path)
        finally: os.close(held)
        path.unlink(); other = self.root / 'other'; other.write_text('preserve'); path.symlink_to(other)
        with self.assertRaises(OSError): updates.locked_file(path)
        self.assertEqual(other.read_text(), 'preserve')

    def fixture_install(self, version):
        source = self.root / ('build-' + version)
        (source / 'ruwac').mkdir(parents=True)
        (source / 'ruwac/Cargo.toml').write_text('[workspace.package]\nversion="' + version + '"\n')
        binary = source / 'target/release/ruwac'; binary.parent.mkdir(parents=True)
        binary.write_text('#!/bin/sh\nprintf "RUWAC ' + version + '\\n"\n'); binary.chmod(0o755)
        original = subprocess.check_output
        def output(args, **kwargs):
            if args[0] == 'git': return (COMMIT + '\n') if kwargs.get('text') else (COMMIT + '\n').encode()
            return original(args, **kwargs)
        with patch.object(sys, 'argv', ['installer', '--build-root', str(source), '--prefix', str(self.root / 'prefix')]), patch.object(subprocess, 'check_output', side_effect=output), contextlib.redirect_stdout(io.StringIO()):
            result = runpy.run_path(str(ROOT / 'scripts/install-user.py'), run_name='__main__')
            os.close(result['lock'])
        return source

    def test_real_install_and_rollback_keep_running_executable_inode(self):
        self.fixture_install('0.0.1')
        prefix = self.root / 'prefix'
        old = (prefix / 'bin/ruwac').resolve()
        old_stat = old.stat()
        self.fixture_install('0.0.2')
        self.assertEqual(updates.installed_version(prefix, '0.0.1'), '0.0.2')
        self.assertEqual(old.stat().st_ino, old_stat.st_ino)
        self.assertEqual(subprocess.check_output([str(old)], text=True).strip(), 'RUWAC 0.0.1')
        self.assertEqual(updates.rollback(prefix)['installed'], '0.0.1')
        self.assertEqual((prefix / 'bin/ruwac').resolve(), old)
        self.assertEqual(updates.rollback(prefix)['installed'], '0.0.2')

    def test_tampered_previous_artifact_refuses_rollback(self):
        self.fixture_install('0.0.1'); self.fixture_install('0.0.2')
        base = self.root / 'prefix/lib/ruwac'
        (base / 'previous/ruwac').write_text('changed')
        before = (base / 'current').readlink()
        with self.assertRaisesRegex(ValueError, 'changed'): updates.rollback(self.root / 'prefix')
        self.assertEqual((base / 'current').readlink(), before)

    def test_newer_running_frontend_does_not_mask_rolled_back_installation(self):
        self.fixture_install('0.0.1'); self.fixture_install('0.0.2')
        prefix = self.root / 'prefix'
        updates.rollback(prefix)
        self.assertEqual(updates.installed_version(prefix, '0.0.2'), '0.0.1')
        data = self.metadata(tag_name='v0.0.2', html_url='https://github.com/w3fans/RUWAC/releases/tag/v0.0.2')
        with patch.object(sys, 'argv', ['updater', '--current', '0.0.2', '--check', '--prefix', str(prefix)]), patch.object(updates, 'api', side_effect=[data, {'object': {'type': 'commit', 'sha': COMMIT}}]), contextlib.redirect_stdout(io.StringIO()) as output:
            updates.main()
        result = json.loads(output.getvalue())
        self.assertTrue(result['available'])
        self.assertEqual(result['current'], '0.0.1')
        self.assertEqual(result['running'], '0.0.2')

    def test_missing_previous_and_foreign_prefix_preserved(self):
        self.fixture_install('0.0.1')
        prefix = self.root / 'prefix'
        with self.assertRaisesRegex(ValueError, 'previous'): updates.rollback(prefix)
        other = self.root / 'foreign'; other.mkdir(); linked = self.root / 'linked'; linked.symlink_to(other)
        with self.assertRaisesRegex(ValueError, 'Unsafe'): updates.installed_version(linked, '0.0.1')
        self.assertTrue(linked.is_symlink())

    def test_build_failure_never_runs_installer(self):
        cache = updates.private_directory(self.root / 'update-cache')
        def fail(args, **kwargs):
            if args[0] == 'cargo': raise subprocess.CalledProcessError(1, args)
        original = Path.read_text
        def text(path, *args, **kwargs):
            return 'ID=fedora\nVERSION_ID=44\n' if str(path) == '/etc/os-release' else original(path, *args, **kwargs)
        with patch.object(Path, 'read_text', text), patch.object(updates, 'checkout'), patch.object(shutil, 'which', return_value='/usr/bin/tool'), patch.object(updates, 'run', side_effect=fail) as run:
            with self.assertRaises(subprocess.CalledProcessError):
                updates.install({'tag': 'v0.2.1', 'latest': '0.2.1', 'commit': COMMIT}, self.root / 'prefix', cache)
            self.assertFalse(any('install-user.py' in str(call.args[0]) for call in run.call_args_list))
        self.assertFalse((self.root / 'prefix').exists())
        self.assertFalse(list(cache.glob('source-*')))

    def test_rollback_refuses_while_an_update_build_is_active(self):
        self.fixture_install('0.0.1'); self.fixture_install('0.0.2')
        cache = updates.private_directory(self.root / 'cache/RUWAC/updates')
        lock = updates.locked_file(cache / '.update.lock')
        try:
            with patch.object(sys, 'argv', ['updater', '--current', '0.0.2', '--rollback', '--prefix', str(self.root / 'prefix')]):
                with self.assertRaisesRegex(ValueError, 'active'): updates.main()
            self.assertEqual(updates.installed_version(self.root / 'prefix', '0.0.0'), '0.0.2')
        finally:
            os.close(lock)


if __name__ == '__main__':
    unittest.main()
