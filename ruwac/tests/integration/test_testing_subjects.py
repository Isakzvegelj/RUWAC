#!/usr/bin/env python3
"""Generated attestation subject coverage and fail-closed artifact regressions."""
import hashlib
from pathlib import Path
import runpy
import tempfile
import unittest

module = runpy.run_path(str(Path(__file__).resolve().parents[3] / 'scripts/testing-subjects.py'))
subjects = module['subjects']


class ArtifactSubjects(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        self.bundles = self.root / 'bundles'
        self.rpms = self.root / 'rpms'
        self.bundles.mkdir()
        self.rpms.mkdir()
        self.archive = self.bundles / 'ruwac-0.2.0-fedora44-x86_64-testing.tar.gz'
        self.core = self.rpms / 'ruwac-0.2.0-1.fc44.x86_64.rpm'
        self.desktop = self.rpms / 'ruwac-desktop-0.2.0-1.fc44.x86_64.rpm'
        for path, data in [(self.archive, b'generated archive'), (self.core, b'generated core'), (self.desktop, b'generated desktop')]:
            path.write_bytes(data)
        self.addCleanup(self.temp.cleanup)

    def test_all_three_distinct_artifacts_are_covered(self):
        self.assertEqual(dict(subjects(self.bundles, self.rpms)), {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in [self.archive, self.core, self.desktop]})

    def test_missing_rpm_does_not_silently_sign_only_archive(self):
        self.desktop.unlink()
        with self.assertRaises(ValueError):
            subjects(self.bundles, self.rpms)

    def test_mixed_versions_are_refused(self):
        self.desktop.rename(self.rpms / 'ruwac-desktop-0.2.1-1.fc44.x86_64.rpm')
        with self.assertRaises(ValueError):
            subjects(self.bundles, self.rpms)

    def test_symlinked_artifact_is_refused(self):
        self.core.unlink()
        self.core.symlink_to(self.archive)
        with self.assertRaises(ValueError):
            subjects(self.bundles, self.rpms)

    def test_extra_artifact_is_refused(self):
        (self.rpms / 'unexpected.rpm').write_bytes(b'generated extra')
        with self.assertRaises(ValueError):
            subjects(self.bundles, self.rpms)

    def test_changed_artifact_changes_its_signed_subject(self):
        before = dict(subjects(self.bundles, self.rpms))
        self.core.write_bytes(b'generated changed core')
        after = dict(subjects(self.bundles, self.rpms))
        self.assertNotEqual(before[self.core.name], after[self.core.name])
        self.assertEqual(before[self.archive.name], after[self.archive.name])


if __name__ == '__main__':
    unittest.main()
