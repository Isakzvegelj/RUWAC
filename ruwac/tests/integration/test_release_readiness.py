#!/usr/bin/env python3
"""Publication eligibility is distinct from installed and running versions."""
import importlib.util
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[3]
spec = importlib.util.spec_from_file_location('updates', ROOT / 'scripts/update-user.py')
u = importlib.util.module_from_spec(spec); spec.loader.exec_module(u)
SHA = 'a' * 40


def release(version):
    return dict(tag='v' + version, latest=version, commit=SHA, available=True)


def metadata(candidate):
    v, tag = candidate['latest'], candidate['tag']
    names = [f'ruwac-{v}-fedora44-x86_64-testing.tar.gz', f'ruwac-{v}-1.fc44.x86_64.rpm', f'ruwac-desktop-{v}-1.fc44.x86_64.rpm']
    names += [name + '.sigstore.json' for name in names]
    return dict(tag_name=tag, draft=False, prerelease=False, assets=[dict(name=name, state='uploaded', size=100,
        digest='sha256:' + 'b' * 64, browser_download_url=f'https://github.com/w3fans/RUWAC/releases/download/{tag}/{name}') for name in names])


class Readiness(unittest.TestCase):
    def test_failed_or_missing_job_has_precise_reason_and_no_advisory_suppression(self):
        run = dict(id=42, head_sha=SHA, head_branch='main', path='.github/workflows/quality.yml')
        for conclusion in ['failure', 'cancelled', None]:
            jobs = [dict(name='quality', conclusion=conclusion, status='in_progress'), dict(name='privacy', conclusion='success')]
            with self.subTest(conclusion=conclusion), patch.object(u, 'api', side_effect=[dict(workflow_runs=[run]), dict(jobs=jobs)]):
                self.assertIn('quality: ' + (conclusion or 'in_progress'), u.quality_failure(release('0.4.28')))
        jobs = [dict(name=n, conclusion='success') for n in ['quality', 'privacy']] + [dict(name='advisories', conclusion='failure')]
        with patch.object(u, 'api', side_effect=[dict(workflow_runs=[run]), dict(jobs=jobs)]):
            self.assertIsNone(u.quality_failure(release('0.4.31')))

    def test_newest_rerun_failure_does_not_reuse_an_old_success(self):
        runs = [dict(id=i, head_sha=SHA, head_branch='main', path='.github/workflows/quality.yml') for i in [1, 2]]
        with patch.object(u, 'api', side_effect=[dict(workflow_runs=runs), dict(jobs=[dict(name='quality', conclusion='failure')])]) as api:
            self.assertIn('quality: failure', u.quality_failure(release('0.4.28')))
            self.assertIn('/2/jobs', api.call_args.args[0])

    def test_incomplete_and_malformed_assets_hold_all_delivery_modes(self):
        candidate = release('0.4.28')
        for delivery in ['source-build', 'testing-rpm', 'testing-bundle']:
            for missing in range(6):
                data = metadata(candidate); removed = data['assets'].pop(missing)['name']
                with patch.object(u, 'quality_failure', return_value=None), patch.object(u, 'api', return_value=data):
                    self.assertIn(removed, u.release_readiness(candidate, delivery))
        for change in [dict(size=0), dict(state='new'), dict(digest='sha256:bad'), dict(browser_download_url='https://example.org/untrusted')]:
            data = metadata(candidate); data['assets'][0].update(change)
            with patch.object(u, 'quality_failure', return_value=None), patch.object(u, 'api', return_value=data):
                self.assertIn('Invalid published asset', u.release_readiness(candidate, 'source-build'))

    def test_failed_release_never_starts_compiler_or_download_even_on_manual_install(self):
        with tempfile.TemporaryDirectory() as root:
            for automatic in [False, True]:
                with patch.object(u, 'quality_failure', return_value='quality: failure'), patch.object(u, 'run') as run, patch.object(u, 'download_bundle') as download:
                    result = u.install(release('0.4.24'), Path(root)/'prefix', Path(root), desktop=False, automatic=automatic)
                    self.assertEqual(result['skipped'], 'quality: failure')
                    self.assertNotIn('installed', result)
                    run.assert_not_called(); download.assert_not_called()

    def test_publication_fallback_running_and_installed_remain_distinct(self):
        newest, complete = release('0.4.28'), release('0.4.27')
        with patch.object(u, 'latest', return_value=newest), patch.object(u, 'newer_releases', return_value=[newest, complete]), patch.object(u, 'release_readiness', side_effect=lambda r,d: 'quality: failure' if r == newest else None):
            result = u.verified_status('0.4.26', 'testing-bundle')
            self.assertEqual((result['latest_published'], result['latest_verified'], result['installed_version']), ('0.4.28','0.4.27','0.4.26'))
            self.assertTrue(result['available'])
            self.assertEqual(result['skipped_releases'][0]['reason'], 'quality: failure')
            self.assertFalse(u.verified_status('0.4.31', 'testing-bundle')['available'])

    def test_mixed_installations_do_not_replace_the_running_frontend_identity(self):
        import subprocess
        with patch.object(u, 'installed_version', return_value='0.4.31'), patch.object(u.subprocess, 'run', return_value=subprocess.CompletedProcess([],0,'0.4.30','')):
            result = u.installation_identity(Path('/generated/prefix'), '0.4.26', '/generated/old/frontend')
        self.assertEqual(result['running'], '0.4.26')
        self.assertEqual(result['running_executable'], '/generated/old/frontend')
        self.assertEqual(result['user_installed'], '0.4.31')
        self.assertEqual(result['system_installed'], '0.4.30')
        self.assertTrue(result['mixed_installation'])

    def test_uninstalled_frontend_and_concurrent_upgrade_are_reported_honestly(self):
        identity=dict(running='0.4.31',running_executable='/generated/build/frontend',user_installed=None,system_installed='0.4.30',mixed_installation=False)
        status=dict(current='0.4.31',latest_verified='0.4.32',available=True)
        result=u.installation_status(status,identity)
        self.assertIsNone(result['installed_version'])
        self.assertEqual(result['running'],'0.4.31')
        identity['user_installed']='0.4.33'
        result=u.installation_status(status,identity)
        self.assertEqual(result['installed_version'],'0.4.33')
        self.assertFalse(result['available'])
        result=u.installation_status(status,identity,managed_rpm=True)
        self.assertEqual(result['installed_version'],'0.4.30')
        self.assertTrue(result['available'])

    def test_no_eligible_release_does_not_claim_up_to_date_or_success(self):
        with patch.object(u, 'latest', return_value=release('0.4.28')), patch.object(u, 'newer_releases', return_value=[]), patch.object(u, 'release_readiness', return_value='Missing published asset: generated.rpm'):
            result = u.verified_status('0.4.24', 'testing-rpm')
            self.assertIsNone(result['latest_verified']); self.assertFalse(result['available'])
            self.assertIn('Missing published asset', result['skipped']); self.assertNotIn('installed', result)


if __name__ == '__main__':
    unittest.main()
