#!/usr/bin/env python3
"""Generated asset/stream/archive faults; no production account, network or installation."""
import gzip
import hashlib
import importlib.util
import io
import json
import os
from pathlib import Path
import tarfile
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[3]
SPEC = importlib.util.spec_from_file_location('bundle_updates', ROOT / 'scripts/update-user.py')
updates = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(updates)
RELEASE = {'tag': 'v0.2.1', 'latest': '0.2.1', 'commit': 'a' * 40}
NAME = 'ruwac-0.2.1-fedora44-x86_64-testing'
URL = 'https://github.com/w3fans/RUWAC/releases/download/v0.2.1/' + NAME + '.tar.gz'


class BundleUpdates(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        self.environment = patch.dict(os.environ, {'XDG_CONFIG_HOME': str(self.root / 'config')})
        self.environment.start()

    def tearDown(self):
        self.environment.stop()
        self.temp.cleanup()

    def archive(self, change=None, extra=None):
        files = {'install-user.py': b'# inert generated installer fixture\n', 'target/release/ruwac': b'generated binary fixture'}
        manifest = {'schema': 1, 'kind': 'fedora-testing-bundle', 'version': '0.2.1', 'source_commit': RELEASE['commit'],
                    'distribution': 'fedora', 'distribution_version': '44', 'architecture': 'x86_64',
                    'security_qualified': False, 'known_advisories': ['RUSTSEC-2023-0071'],
                    'files': {name: hashlib.sha256(data).hexdigest() for name, data in files.items()}}
        if change:
            change(manifest, files)
        files['bundle.json'] = json.dumps(manifest).encode()
        output = self.root / ('archive-' + str(len(list(self.root.iterdir()))) + '.tar.gz')
        with tarfile.open(output, 'w:gz') as archive:
            for name, data in files.items():
                info = tarfile.TarInfo(NAME + '/' + name)
                info.size = len(data)
                archive.addfile(info, io.BytesIO(data))
            if extra:
                archive.addfile(extra)
        return output

    def asset(self, payload=b'generated payload'):
        return {'name': NAME + '.tar.gz', 'state': 'uploaded', 'size': len(payload),
                'browser_download_url': URL, 'digest': 'sha256:' + hashlib.sha256(payload).hexdigest()}

    def test_delivery_is_explicit_and_preserves_other_preferences(self):
        self.assertFalse(updates.testing_bundle_setting())
        updates.save_preferences({'schema': 1, 'automatic_check': False, 'automatic_install': False})
        self.assertTrue(updates.testing_bundle_setting(True))
        self.assertFalse(updates.preferences()['automatic_check'])
        self.assertFalse(updates.testing_bundle_setting(False))

    def test_asset_identity_size_digest_and_ambiguity(self):
        data = {'tag_name': RELEASE['tag'], 'draft': False, 'prerelease': False, 'assets': []}
        with patch.object(updates, 'api', return_value=data):
            self.assertIsNone(updates.bundle_asset(RELEASE))
        good = self.asset()
        data['assets'] = [good]
        with patch.object(updates, 'api', return_value=data):
            self.assertEqual(updates.bundle_asset(RELEASE), good)
        for change in [{'size': 0}, {'size': True}, {'size': updates.MAX_BUNDLE_BYTES + 1}, {'state': 'starter'},
                       {'digest': ''}, {'digest': 'sha256:' + 'z' * 64}, {'browser_download_url': URL + '?other'},
                       {'browser_download_url': 'https://example.invalid/payload'}]:
            data['assets'] = [dict(good, **change)]
            with patch.object(updates, 'api', return_value=data), self.assertRaises(ValueError):
                updates.bundle_asset(RELEASE)
        data['assets'] = [good, good]
        with patch.object(updates, 'api', return_value=data), self.assertRaises(ValueError):
            updates.bundle_asset(RELEASE)

    def test_redirects_reject_foreign_hosts_http_credentials_and_fragments(self):
        for url in ['http://github.com/payload', 'https://example.invalid/payload',
                    'https://github.com.example.invalid/payload', 'https://user@github.com/payload',
                    'https://github.com:444/payload', 'https://github.com/payload#fragment']:
            self.assertFalse(updates.asset_url_allowed(url))
        self.assertTrue(updates.asset_url_allowed(URL))
        self.assertTrue(updates.asset_url_allowed('https://release-assets.githubusercontent.com/generated?signature=generated'))

    def test_download_hash_size_and_response_origin_before_unpack(self):
        payload = b'generated download body'
        asset = self.asset(payload)
        class Response(io.BytesIO):
            url = URL
        for body, url, passes in [(payload, URL, True), (payload[:-1], URL, False),
                                  (payload + b'!', URL, False), (b'X' * len(payload), URL, False),
                                  (payload, 'https://example.invalid/body', False)]:
            response = Response(body)
            response.url = url
            opener = unittest.mock.Mock()
            opener.open.return_value = response
            output = self.root / ('download-' + str(len(list(self.root.iterdir()))))
            with patch.object(updates.urllib.request, 'build_opener', return_value=opener):
                if passes:
                    updates.download_bundle(asset, output)
                    self.assertEqual(output.read_bytes(), payload)
                else:
                    with self.assertRaises(ValueError):
                        updates.download_bundle(asset, output)

    def test_archive_rejects_traversal_links_special_entries_and_duplicates(self):
        for name, kind in [(NAME + '/../outside', tarfile.REGTYPE), ('/outside', tarfile.REGTYPE),
                           (NAME + '/link', tarfile.SYMTYPE), (NAME + '/hard', tarfile.LNKTYPE),
                           (NAME + '/fifo', tarfile.FIFOTYPE), (NAME + '/install-user.py', tarfile.REGTYPE)]:
            info = tarfile.TarInfo(name)
            info.type = kind
            info.linkname = '/outside'
            with self.assertRaises((ValueError, tarfile.TarError)):
                updates.unpack_bundle(self.archive(extra=info), self.root / ('unpacked-' + str(len(list(self.root.iterdir())))), RELEASE)
        self.assertFalse((self.root / 'outside').exists())

    def test_manifest_commit_version_qualification_inventory_and_checksums(self):
        for change in [lambda m,f: m.update(source_commit='b' * 40), lambda m,f: m.update(version='0.2.2'),
                       lambda m,f: m.update(security_qualified=True), lambda m,f: m.update(known_advisories=[]),
                       lambda m,f: m.update(architecture='other'), lambda m,f: f.update(extra=b'extra'),
                       lambda m,f: f.update({'install-user.py': b'tampered'}),
                       lambda m,f: m['files'].update({'install-user.py': 'z' * 64})]:
            with self.assertRaises(ValueError):
                updates.unpack_bundle(self.archive(change), self.root / ('unpacked-' + str(len(list(self.root.iterdir())))), RELEASE)
        extracted = updates.unpack_bundle(self.archive(), self.root / 'valid', RELEASE)
        self.assertEqual(extracted.name, NAME)

    def test_expansion_budget_is_enforced_before_installation(self):
        with patch.object(updates, 'MAX_UNPACKED_BYTES', 100), self.assertRaises(ValueError):
            updates.unpack_bundle(self.archive(), self.root / ('unpacked-' + str(len(list(self.root.iterdir())))), RELEASE)

    def test_bundle_choice_waits_for_assets_and_quality_without_compiler(self):
        updates.testing_bundle_setting(True)
        for asset, quality in [(None, True), (self.asset(), False)]:
            with patch.object(Path, 'read_text', return_value='ID=fedora\nVERSION_ID=44\n'), patch.object(updates, 'testing_bundle_setting', return_value=True), patch.object(updates, 'bundle_asset', return_value=asset), patch.object(updates, 'automatic_release_verified', return_value=quality), patch.object(updates.platform, 'machine', return_value='x86_64'), patch.object(updates.shutil, 'which') as compiler:
                result = updates.install(RELEASE, self.root / 'prefix', self.root / 'cache')
                self.assertIn('skipped', result)
                compiler.assert_not_called()


    def test_remote_prebuilt_install_uses_bundle_without_preference_or_compiler(self):
        with patch.object(Path, 'read_text', return_value='ID=fedora\nVERSION_ID=44\n'), \
                patch.object(updates, 'testing_bundle_setting', return_value=False), \
                patch.object(updates, 'bundle_asset', return_value=self.asset()), \
                patch.object(updates, 'automatic_release_verified', return_value=True), \
                patch.object(updates.platform, 'machine', return_value='x86_64'), \
                patch.object(updates, 'install_testing_bundle', return_value={'installed': '0.1.2'}) as bundle, \
                patch.object(updates.shutil, 'which') as compiler:
            result = updates.install(RELEASE, self.root / 'prefix', self.root / 'cache', desktop=False, prebuilt=True)
        self.assertEqual(result, {'installed': '0.1.2'})
        self.assertEqual(bundle.call_args.args[5:], (False,))
        self.assertTrue(bundle.call_args.kwargs['prebuilt'])
        compiler.assert_not_called()

if __name__ == '__main__':
    unittest.main()
