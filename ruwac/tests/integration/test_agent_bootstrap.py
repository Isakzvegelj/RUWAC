#!/usr/bin/env python3
"""Generated installer contracts; never downloads or authenticates real agents."""
import importlib.util
import io
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location('bootstrap', Path(__file__).resolve().parents[3] / 'scripts/agent-bootstrap.py')
bootstrap = importlib.util.module_from_spec(spec)
spec.loader.exec_module(bootstrap)


class BootstrapTests(unittest.TestCase):
    def launch(self, provider, command, present):
        env = {'PATH': '/usr/bin', 'HOME': '/nonexistent'}
        with patch.object(sys, 'argv', ['bootstrap', provider, command]), patch.dict(os.environ, env, clear=True), patch.object(bootstrap, 'available', side_effect=present) as available, patch.object(bootstrap, 'install') as install, patch.object(os, 'execvpe') as execute:
            bootstrap.main()
        return available, install, execute

    def test_existing_providers_never_install(self):
        for provider, command in [('codex', 'codex resume --all'), ('claude', 'claude --resume'), ('kimi', 'kimi --session')]:
            available, install, execute = self.launch(provider, command, [True])
            install.assert_not_called()
            self.assertEqual(available.call_args.args[0], provider)
            self.assertTrue(execute.call_args.args[1][-1].endswith(command))

    def test_missing_providers_install_once_then_launch(self):
        for provider in bootstrap.INSTALLERS:
            available, install, execute = self.launch(provider, provider, [False, True])
            self.assertEqual(available.call_count, 2)
            install.assert_called_once()
            self.assertEqual(install.call_args.args[0], provider)
            self.assertIn('.local/bin', execute.call_args.args[2]['PATH'])
            self.assertIn('.kimi-code/bin', execute.call_args.args[2]['PATH'])

    def test_failed_install_does_not_launch(self):
        with self.assertRaises(ValueError):
            self.launch('kimi', 'kimi --session', [False, False])

    def test_invalid_commands_rejected_before_install(self):
        for command in ['', '\n', 'a' * 1025, 'unterminated"']:
            with self.assertRaises(ValueError):
                self.launch('kimi', command, [])

    def test_unknown_provider_rejected(self):
        with self.assertRaises(ValueError):
            self.launch('unknown', 'unknown', [])

    def test_https_download_complete_private_and_bounded(self):
        data = b'#!/bin/sh\nexit 0\n'
        response = io.BytesIO(data)
        response.url = bootstrap.INSTALLERS['kimi'][0]
        observed = []

        def run(argv, **kwargs):
            path = Path(argv[1])
            self.assertEqual(path.read_bytes(), data)
            self.assertEqual(path.stat().st_mode & 0o777, 0o600)
            self.assertEqual(kwargs['timeout'], 600)
            self.assertTrue(kwargs['check'])
            observed.append(path)

        with patch.object(bootstrap.urllib.request, 'urlopen', return_value=response) as fetch, patch.object(subprocess, 'run', side_effect=run):
            bootstrap.install('kimi', {'PATH': '/usr/bin'})
        self.assertEqual(fetch.call_args.kwargs['timeout'], 30)
        self.assertFalse(observed[0].exists())

    def test_invalid_downloads_never_execute(self):
        for url, data in [('http://example.test', b'generated'), ('https://example.test', b''), ('https://example.test', b'a' * (1024 * 1024 + 1))]:
            response = io.BytesIO(data)
            response.url = url
            with patch.object(bootstrap.urllib.request, 'urlopen', return_value=response), patch.object(subprocess, 'run') as run:
                with self.assertRaises(ValueError):
                    bootstrap.install('kimi', {})
                run.assert_not_called()

    def test_download_failure_never_executes(self):
        with patch.object(bootstrap.urllib.request, 'urlopen', side_effect=OSError('generated')), patch.object(subprocess, 'run') as run:
            with self.assertRaises(OSError):
                bootstrap.install('codex', {})
            run.assert_not_called()

    def test_alias_probe_uses_positional_argument(self):
        with patch.object(bootstrap.shutil, 'which', return_value=None), patch.object(subprocess, 'run', return_value=subprocess.CompletedProcess([], 0)) as run:
            self.assertTrue(bootstrap.available('generated-alias', {'PATH': '/usr/bin'}))
        self.assertEqual(run.call_args.args[0][-1], 'generated-alias')
        self.assertEqual(run.call_args.kwargs['timeout'], 10)


if __name__ == '__main__':
    unittest.main()
