#!/usr/bin/env python3
"""Wait for exact-commit source/privacy checks; transient API errors never pass."""
import argparse
import json
import os
from pathlib import Path
import re
import runpy
import subprocess
import time


class RetryableAPIError(RuntimeError):
    pass


class QualityFailure(RuntimeError):
    pass


class GitHubAPI:
    def __init__(self, repository, deadline, gh='gh'):
        if not re.fullmatch(r'[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+', repository):
            raise ValueError('Invalid repository identity')
        self.repository, self.deadline, self.gh = repository, deadline, gh

    def __call__(self, path):
        remaining = self.deadline - time.monotonic()
        if remaining <= 0:
            raise RetryableAPIError('Quality deadline reached')
        try:
            result = subprocess.run([self.gh, 'api', 'repos/' + self.repository + path],
                                    capture_output=True, text=True, timeout=min(30, remaining))
        except subprocess.TimeoutExpired as error:
            raise RetryableAPIError('GitHub API timeout') from error
        if result.returncode:
            status = re.search(r'HTTP (\d{3})', result.stderr)
            if status and int(status[1]) in {429, 500, 502, 503, 504}:
                raise RetryableAPIError('Transient GitHub API HTTP ' + status[1])
            if not status and any(term in result.stderr.lower() for term in
                                  ['timeout', 'connection reset', 'unexpected eof']):
                raise RetryableAPIError('Transient GitHub API transport error')
            # Do not echo authentication diagnostics, response bodies or tokens.
            raise RuntimeError('GitHub API access failed' + (' (HTTP ' + status[1] + ')' if status else ''))
        data = json.loads(result.stdout)
        if '/jobs?' in path:
            for job in data.get('jobs', []):
                if job.get('name') in {'quality', 'privacy'} and job.get('status') == 'completed' and job.get('conclusion') != 'success':
                    raise QualityFailure('Required source/privacy job failed; publication held')
        return data


def wait(verify, deadline, interval=20, clock=time.monotonic, sleeper=time.sleep):
    while clock() < deadline:
        try:
            if verify():
                return
        except RetryableAPIError:
            print('Transient GitHub API failure; exact-commit checks remain unverified', flush=True)
        remaining = deadline - clock()
        if remaining > 0:
            sleeper(min(interval, remaining))
    raise TimeoutError('Exact-commit source/privacy checks did not pass before the deadline')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--repository', default=os.environ.get('GITHUB_REPOSITORY'))
    parser.add_argument('--commit', default=os.environ.get('GITHUB_SHA'))
    parser.add_argument('--timeout', type=float, default=1200)
    args = parser.parse_args()
    if not args.repository or not args.commit or args.timeout <= 0:
        parser.error('Repository, immutable commit and a positive timeout are required')
    deadline = time.monotonic() + args.timeout
    verify = runpy.run_path(str(Path(__file__).with_name('update-user.py')))['automatic_release_verified']
    verify.__globals__['api'] = GitHubAPI(args.repository, deadline)
    wait(lambda: verify({'commit': args.commit}), deadline)
    print('Exact-commit source/privacy checks PASS; dependency advisories remain separate')


if __name__ == '__main__':
    main()
