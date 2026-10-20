#!/usr/bin/env python3
"""Publication refuses missing gates, changed artifacts, evidence and staged source."""
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[3]
spec = importlib.util.spec_from_file_location('acceptance', ROOT / 'scripts/acceptance-gate.py')
a = importlib.util.module_from_spec(spec); spec.loader.exec_module(a)


class PublicationGate(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(dir=os.environ.get('RUWAC_TEST_RUN_ROOT'))
        self.addCleanup(self.temporary.cleanup)
        self.base = Path(self.temporary.name)
        self.root = self.base/'build'; self.root.mkdir()
        (self.root/'ruwac/desktop').mkdir(parents=True)
        (self.root/'README.md').write_text('Generated fixture\n')
        (self.root/'ruwac/Cargo.toml').write_text('[workspace.package]\nversion="9.0.1"\n')
        (self.root/'ruwac/desktop/Cargo.toml').write_text('[package]\nversion="9.0.1"\n')
        (self.root/'target/release').mkdir(parents=True)
        for name in ['ruwac','ruwac-desktop']:
            (self.root/'target/release'/name).write_bytes(b'generated artifact '+name.encode())
        self.proof = self.root/'evidence.json'; self.proof.write_text('{"generated":true}\n')
        private = self.base/'build-private'; private.mkdir(mode=0o700)
        self.receipt = private/'acceptance.json'
        self.data = dict(schema=1,status='PASS',version='9.0.1',source=a.sources(self.root),
            binaries={n:a.digest(self.root/'target/release'/n)for n in ['ruwac','ruwac-desktop']},
            gates=[dict(name=n,status='PASS',evidence=[dict(path=str(self.proof),sha256=a.digest(self.proof))])for n in sorted(a.REQUIRED)])

    def write(self):
        self.receipt.write_text(json.dumps(self.data)); self.receipt.chmod(0o600)

    def test_exact_artifacts_and_required_proofs_accept_optional_external_block(self):
        self.data['gates'].append(dict(name='physical-wayland-input',status='BLOCKED'))
        self.write()
        self.assertEqual(a.validate(self.root,self.receipt)['status'],'PASS')

    def test_missing_or_unexecuted_required_gate_is_never_publication_success(self):
        for state in ['NOT RUN','BLOCKED','FAIL','PASS (preflight)',True]:
            with self.subTest(state=state):
                self.data['gates'][0]['status']=state; self.write()
                with self.assertRaisesRegex(ValueError,'have not passed'):a.validate(self.root,self.receipt)
        self.data['gates'].pop(0);self.write()
        with self.assertRaisesRegex(ValueError,'have not passed'):a.validate(self.root,self.receipt)

    def test_changed_source_or_binary_refuses_the_same_old_receipt(self):
        self.write();path=self.root/'README.md';before=path.read_bytes();path.write_text('changed')
        with self.assertRaisesRegex(ValueError,'Source changed'):a.validate(self.root,self.receipt)
        path.write_bytes(before);(self.root/'target/release/ruwac').write_text('different binary')
        with self.assertRaisesRegex(ValueError,'binary changed'):a.validate(self.root,self.receipt)

    def test_wrong_version_and_duplicate_gate_refuse(self):
        self.data['version']='9.0.2';self.write()
        with self.assertRaisesRegex(ValueError,'versions differ'):a.validate(self.root,self.receipt)
        self.data['version']='9.0.1';self.data['gates'].append(self.data['gates'][0]);self.write()
        with self.assertRaisesRegex(ValueError,'duplicated'):a.validate(self.root,self.receipt)

    def test_changed_missing_or_symlinked_evidence_refuses(self):
        self.write();before=self.proof.read_bytes();self.proof.write_text('changed evidence')
        with self.assertRaisesRegex(ValueError,'evidence changed'):a.validate(self.root,self.receipt)
        self.proof.write_bytes(before)
        link=self.root/'proof-link';link.symlink_to(self.proof)
        self.data['gates'][0]['evidence'][0]['path']=str(link);self.write()
        with self.assertRaisesRegex(ValueError,'without symlinks'):a.validate(self.root,self.receipt)
        self.data['gates'][0]['evidence']=[];self.write()
        with self.assertRaisesRegex(ValueError,'evidence is missing'):a.validate(self.root,self.receipt)

    def test_later_staged_edit_cannot_bypass_frozen_source(self):
        subprocess.run(['git','init','-q',str(self.root)],check=True)
        subprocess.run(['git','-C',str(self.root),'add','README.md','ruwac/Cargo.toml','ruwac/desktop/Cargo.toml'],check=True)
        self.write();self.assertEqual(a.validate(self.root,self.receipt,staged=True)['status'],'PASS')
        (self.root/'README.md').write_text('working tree tested later, index remains older\n')
        self.data['source']=a.sources(self.root);self.write()
        with self.assertRaisesRegex(ValueError,'Staged source differs'):a.validate(self.root,self.receipt,staged=True)


if __name__=='__main__':unittest.main()
