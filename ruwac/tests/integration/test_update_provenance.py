#!/usr/bin/env python3
"""Generated release metadata and real verifier subprocess faults; no owner data."""
import importlib.util
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[3]
SPEC = importlib.util.spec_from_file_location('updates', ROOT / 'scripts/update-user.py')
updates = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(updates)


class Provenance(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        self.release = {'latest': '0.2.1', 'tag': 'v0.2.1', 'commit': 'a' * 40}
        name = 'ruwac-0.2.1-fedora44-x86_64-testing.tar.gz'
        url = 'https://github.com/w3fans/RUWAC/releases/download/v0.2.1/' + name
        self.asset = {'name': name, 'browser_download_url': url}
        self.signature = {'name': name + '.sigstore.json', 'browser_download_url': url + '.sigstore.json',
                          'state': 'uploaded', 'size': 30, 'digest': 'sha256:' + 'b' * 64}
        self.data = {'tag_name': 'v0.2.1', 'draft': False, 'prerelease': False, 'assets': [self.signature]}
        self.archive = self.root / 'archive.tar.gz'
        self.archive.write_bytes(b'generated subject')
        self.verifier = self.root / 'gh'
        self.verifier.write_text('''#!/usr/bin/env python3
import json, os, sys
from pathlib import Path
Path(__file__).with_suffix('.receipt').write_text(json.dumps({'args':sys.argv[1:], 'config':os.environ['GH_CONFIG_DIR'], 'tokens':[k for k in os.environ if k in ['GH_TOKEN','GITHUB_TOKEN','GH_ENTERPRISE_TOKEN','GITHUB_ENTERPRISE_TOKEN','GH_HOST','GH_REPO','GH_DEBUG']]}))
sys.exit(int(Path(__file__).with_suffix('.status').read_text()))
''')
        self.verifier.chmod(0o700)
        self.verifier.with_suffix('.status').write_text('0')

    def tearDown(self):
        self.temp.cleanup()

    def verify(self):
        with patch.object(updates.shutil, 'which', return_value=str(self.verifier)), \
                patch.object(updates, 'api', return_value=self.data), \
                patch.object(updates, 'download_bundle', side_effect=lambda a,p:p.write_text('{}')):
            updates.verify_bundle_provenance(self.release, self.asset, self.archive, self.root)

    def test_real_process_uses_exact_identity_and_no_owner_credentials(self):
        keys = ['GH_TOKEN','GITHUB_TOKEN','GH_ENTERPRISE_TOKEN','GITHUB_ENTERPRISE_TOKEN','GH_HOST','GH_REPO','GH_DEBUG']
        with patch.dict(os.environ, {k:'generated' for k in keys}):
            self.verify()
        receipt = json.loads(self.verifier.with_suffix('.receipt').read_text())
        self.assertEqual(receipt['tokens'], [])
        args = receipt['args']
        for flag,value in [('--repo','w3fans/RUWAC'), ('--source-ref','refs/tags/v0.2.1'),
                           ('--source-digest','a'*40), ('--signer-workflow','w3fans/RUWAC/.github/workflows/release.yml')]:
            self.assertEqual(args[args.index(flag)+1], value)
        self.assertIn('--deny-self-hosted-runners', args)
        self.assertEqual(Path(receipt['config']).stat().st_mode & 0o777, 0o700)

    def test_failed_verifier_prevents_unpack_and_installer(self):
        self.verifier.with_suffix('.status').write_text('1')
        cache = self.root / 'cache'; cache.mkdir()
        with patch.object(updates.shutil, 'which', return_value=str(self.verifier)), \
                patch.object(updates, 'api', return_value=self.data), \
                patch.object(updates, 'download_bundle', side_effect=lambda a,p:p.write_text('{}')), \
                patch.object(updates, 'unpack_bundle') as unpack, patch.object(updates, 'run') as install:
            with self.assertRaisesRegex(ValueError, 'verification failed'):
                updates.install_testing_bundle(self.release, self.asset, self.root / 'prefix', cache, False)
            unpack.assert_not_called(); install.assert_not_called()
        self.assertEqual([p.name for p in cache.iterdir()], ['.update.lock'])

    def test_missing_ambiguous_or_wrong_signature_held(self):
        cases = [[], [self.signature,self.signature]]
        for changed in [{'state':'starter'}, {'size':True}, {'size':1024*1024+1},
                        {'digest':'sha256:'+'z'*64}, {'browser_download_url':'https://example.invalid/generated'}]:
            cases.append([dict(self.signature, **changed)])
        for assets in cases:
            with self.subTest(assets=assets):
                self.data['assets'] = assets
                with self.assertRaises(ValueError): self.verify()
        self.assertFalse(self.verifier.with_suffix('.receipt').exists())

    def test_missing_verifier_holds_installation(self):
        with patch.object(updates.shutil, 'which', return_value=None), self.assertRaisesRegex(ValueError, 'install gh'):
            updates.verify_bundle_provenance(self.release, self.asset, self.archive, self.root)


if __name__ == '__main__':
    unittest.main()
