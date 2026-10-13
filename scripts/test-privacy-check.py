#!/usr/bin/env python3
"""Generated examples only: this test must never embed deployment identifiers."""
import importlib.util
import json
import os
import random
import string
import subprocess
from pathlib import Path
import tempfile
import unittest
import uuid

spec = importlib.util.spec_from_file_location('privacy', Path(__file__).with_name('privacy-check.py'))
privacy = importlib.util.module_from_spec(spec)
spec.loader.exec_module(privacy)


class PrivacyTests(unittest.TestCase):
    def test_synthetic_private_policy_and_boundaries(self):
        host = 'generated-' + uuid.uuid4().hex
        self.assertTrue(privacy.findings('host=' + host, [host]))
        self.assertTrue(privacy.findings('test_' + host + '_login', [host]))
        self.assertFalse(privacy.findings('prefix' + host + 'suffix', [host]))

    def test_report_never_contains_payload(self):
        value = 'generated-' + uuid.uuid4().hex
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / 'fixture.txt').write_text(value)
            with self.assertRaises(ValueError) as raised:
                privacy.verify(root, ['fixture.txt'], [value])
            self.assertNotIn(value, str(raised.exception))
            self.assertIn('private-policy', str(raised.exception))

    def test_private_addresses_and_reserved_examples(self):
        address = '.'.join(map(str, [10, 23, 47, 61]))
        self.assertTrue(privacy.findings(address))
        for address in ['127.0.0.1', '192.0.2.15', '198.51.100.21', '203.0.113.5']:
            self.assertFalse(privacy.findings(address))

    def test_credentials_and_paths(self):
        examples = ['-----BEGIN ' + 'PRIVATE KEY-----', 'https://' + 'generated:generated-password@host.example.test', 'ghp_' + 'A' * 36, '/home/' + 'generated-user/file', '/run/user/' + '12345/bus']
        for value in examples:
            self.assertTrue(privacy.findings(value))

    def test_safe_symbolic_metadata(self):
        self.assertFalse(privacy.findings('password = resolve_secret(reference)\nhost = profile.server\nuser@example.test\n${HOME}/file'))

    def test_missing_required_policy(self):
        import os
        from unittest.mock import patch
        with patch.dict(os.environ, {}, clear=True):
            with self.assertRaises(ValueError):
                privacy.private_terms(Path('/'), required=True)

    def test_external_policy_permissions(self):
        with tempfile.TemporaryDirectory() as tmp:
            folder = Path(tmp)
            root = folder / 'repo'
            root.mkdir()
            path = folder / 'policy.json'
            term = 'generated-' + uuid.uuid4().hex
            path.write_text(json.dumps({'terms': [term]}))
            path.chmod(0o600)
            self.assertEqual(privacy.private_terms(root, path), [term])
            path.chmod(0o644)
            with self.assertRaises(ValueError):
                privacy.private_terms(root, path)
            path = root / 'policy.json'
            path.write_text(json.dumps({'terms': [term]}))
            path.chmod(0o600)
            with self.assertRaises(ValueError):
                privacy.private_terms(root, path)


class OutgoingHistoryTests(unittest.TestCase):
    def test_intermediate_snapshot_is_checked(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            def git(*args):
                return subprocess.check_output(['git', '-C', tmp, *args], stderr=subprocess.DEVNULL).decode().strip()
            git('init', '-q')
            git('config', 'user.name', 'Generated test')
            git('config', 'user.email', 'test@example.test')
            file = root / 'fixture.txt'
            file.write_text('portable example')
            git('add', '.')
            git('commit', '-qm', 'Generated baseline')
            base = git('rev-parse', 'HEAD')
            term = 'generated-' + uuid.uuid4().hex
            file.write_text(term)
            git('commit', '-qam', 'Generated intermediate')
            file.write_text('portable example')
            git('commit', '-qam', 'Generated clean tip')
            self.assertFalse(privacy.findings(file.read_text(), [term]))
            with self.assertRaises(ValueError) as caught:
                privacy.verify_outgoing(root, base, [term])
            self.assertNotIn(term, str(caught.exception))

    def test_sensitive_filename_is_not_echoed(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            term = 'generated-' + uuid.uuid4().hex
            name = term + '.txt'
            (root / name).write_text('portable example')
            with self.assertRaises(ValueError) as caught:
                privacy.verify(root, [name], [term])
            self.assertNotIn(term, str(caught.exception))


@unittest.skipUnless(os.environ.get('RUWAC_GITLEAKS'), 'Standard scanner supplied in publication/CI')
class StandardScannerTests(unittest.TestCase):
    def scan(self, path):
        return subprocess.run([os.environ['RUWAC_GITLEAKS'], 'dir', str(path), '--config', str(Path(__file__).with_name('gitleaks.toml')), '--redact', '--no-banner'], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL).returncode

    def test_generated_token_is_rejected(self):
        with tempfile.TemporaryDirectory() as tmp:
            # Deliberately generated and never issued by any service.
            value = 'ghp_' + ''.join(random.SystemRandom().choice(string.ascii_letters + string.digits) for _ in range(36))
            Path(tmp, 'example.txt').write_text(value)
            self.assertEqual(self.scan(tmp), 1)

    def test_checksum_exception_is_narrow(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / 'vendor/ironrdp-graphics/UPSTREAM.json'
            path.parent.mkdir(parents=True)
            path.write_text('  "src/zgfx/api.rs": "' + uuid.uuid4().hex + uuid.uuid4().hex + '",\n')
            self.assertEqual(self.scan(tmp), 0)
            # A generated service token in that same provenance file still fails.
            value = 'ghp_' + ''.join(random.SystemRandom().choice(string.ascii_letters + string.digits) for _ in range(36))
            path.write_text(json.dumps({'credential': value}))
            self.assertEqual(self.scan(tmp), 1)

    def test_nonprovenance_hash_is_not_exempted(self):
        with tempfile.TemporaryDirectory() as tmp:
            Path(tmp, 'configuration.json').write_text('  "src/zgfx/api.rs": "' + uuid.uuid4().hex + uuid.uuid4().hex + '",\n')
            self.assertEqual(self.scan(tmp), 1)


if __name__ == '__main__':
    unittest.main()
