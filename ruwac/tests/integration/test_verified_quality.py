#!/usr/bin/env python3
"""Generated GitHub transport/process faults; no live repository writes."""
import contextlib
import importlib.util
import io
import json
from pathlib import Path
import runpy
import sys
import tempfile
import time
import unittest

scripts = Path(__file__).resolve().parents[3] / 'scripts'
spec = importlib.util.spec_from_file_location('verified_quality', scripts / 'verified-quality.py')
q = importlib.util.module_from_spec(spec)
spec.loader.exec_module(q)


class QualityTests(unittest.TestCase):
    def fake_gh(self, root, response, status=None):
        path = root / 'generated-gh'
        path.write_text('#!' + sys.executable + '\nimport sys,json\n' +
                        ('print("gh: generated-private-diagnostic HTTP ' + str(status) + '",file=sys.stderr)\nsys.exit(1)\n' if status else
                         'print(' + repr(json.dumps(response)) + ')\n'))
        path.chmod(0o700)
        return q.GitHubAPI('generated/product', time.monotonic() + 10, str(path))

    def test_real_process_transient_errors_hold_publication(self):
        with tempfile.TemporaryDirectory() as name:
            for status in [429, 500, 502, 503, 504]:
                api = self.fake_gh(Path(name), {}, status)
                with self.assertRaises(q.RetryableAPIError):
                    api('/actions/runs')

    def test_auth_failures_are_terminal_without_raw_diagnostics(self):
        with tempfile.TemporaryDirectory() as name:
            for status in [401, 403, 404]:
                api = self.fake_gh(Path(name), {}, status)
                with self.assertRaises(RuntimeError) as raised:
                    api('/actions/runs')
                self.assertNotIsInstance(raised.exception, q.RetryableAPIError)
                self.assertNotIn('generated-private-diagnostic', str(raised.exception))

    def test_required_failed_job_stops_but_advisory_stays_separate(self):
        with tempfile.TemporaryDirectory() as name:
            for job in ['quality', 'privacy']:
                api = self.fake_gh(Path(name), {'jobs': [{'name': job, 'status': 'completed', 'conclusion': 'failure'}]})
                with self.assertRaises(q.QualityFailure):
                    api('/actions/runs/1/jobs?per_page=100')
            data = {'jobs': [{'name': 'advisories', 'status': 'completed', 'conclusion': 'failure'}]}
            api = self.fake_gh(Path(name), data)
            self.assertEqual(api('/actions/runs/1/jobs?per_page=100'), data)

    def test_transient_then_verified_success_and_deadline(self):
        elapsed = [0]
        calls = [0]
        def verify():
            calls[0] += 1
            if calls[0] < 3:
                raise q.RetryableAPIError('generated')
            return True
        def sleep(n):
            elapsed[0] += n
        with contextlib.redirect_stdout(io.StringIO()):
            q.wait(verify, 10, interval=2, clock=lambda: elapsed[0], sleeper=sleep)
            self.assertEqual(calls[0], 3)
            self.assertEqual(elapsed[0], 4)
            with self.assertRaises(TimeoutError):
                q.wait(lambda: False, 7, interval=2, clock=lambda: elapsed[0], sleeper=sleep)
        self.assertEqual(elapsed[0], 7)

    def test_real_process_recovery_keeps_exact_commit_and_both_checks(self):
        commit = 'a' * 40
        verify = runpy.run_path(str(scripts / 'update-user.py'))['automatic_release_verified']
        with tempfile.TemporaryDirectory() as name:
            root = Path(name)
            gh = root / 'generated-gh'
            gh.write_text('#!' + sys.executable + '\n' + '''import json,pathlib,sys
p=pathlib.Path(__file__).with_name('count');n=int(p.read_text()) if p.exists() else 0;p.write_text(str(n+1))
if n==0:print('gh: Server Error (HTTP 502)',file=sys.stderr);sys.exit(1)
if '/jobs?' in sys.argv[-1]:print(json.dumps({'jobs':[{'name':x,'status':'completed','conclusion':'success' if x!='advisories' else 'failure'} for x in ['quality','privacy','advisories']]}))
else:print(json.dumps({'workflow_runs':[{'id':1,'head_sha':'a'*40,'head_branch':'main','path':'.github/workflows/quality.yml'}]}))
''')
            gh.chmod(0o700)
            verify.__globals__['api'] = q.GitHubAPI('generated/product', time.monotonic() + 5, str(gh))
            with contextlib.redirect_stdout(io.StringIO()):
                q.wait(lambda: verify({'commit': commit}), time.monotonic() + 5, interval=.01)
            self.assertFalse(verify({'commit': 'b' * 40}))


if __name__ == '__main__':
    unittest.main()
