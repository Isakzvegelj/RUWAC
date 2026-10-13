#!/usr/bin/env python3
"""Explicit real-user-manager fixture; unique units and disposable update state."""
import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time
import uuid

ROOT = Path(__file__).resolve().parents[3]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--run-owned-fixture', action='store_true', required=True)
    args = parser.parse_args()
    del args
    spec = importlib.util.spec_from_file_location('updates', ROOT / 'scripts/update-user.py')
    updates = importlib.util.module_from_spec(spec); spec.loader.exec_module(updates)
    manager_dir = Path(os.environ.get('XDG_CONFIG_HOME', Path.home() / '.config')) / 'systemd/user'
    manager_dir.mkdir(parents=True, exist_ok=True)
    info = manager_dir.lstat()
    if manager_dir.is_symlink() or info.st_uid != os.geteuid() or info.st_mode & 0o022:
        raise ValueError('Unsafe user-manager configuration; fixture not started')
    unique = 'ruwac-update-fixture-' + uuid.uuid4().hex
    updates.UNIT_NAMES = (unique + '.service', unique + '.timer')
    published = updates.latest('0.0.0')
    baseline = published['latest']
    units = {}
    with tempfile.TemporaryDirectory(prefix='ruwac-update-fixture-') as temporary:
        base = Path(temporary); prefix = base / 'prefix'
        # This generated launcher runs the real helper through the actual service.
        # It does not replace a user's RUWAC executable or build source artifacts.
        root = prefix / 'lib/ruwac'; release = root / ('releases/' + baseline)
        release.mkdir(parents=True); (root / 'current').symlink_to('releases/' + baseline)
        binary = release / 'ruwac'
        binary.write_text('#!/usr/bin/python3\nimport runpy,sys\nsys.argv=["updater","--current",' + repr(baseline) + ']+sys.argv[2:]\nrunpy.run_path(' + repr(str(ROOT / 'scripts/update-user.py')) + ',run_name="__main__")\n')
        binary.chmod(0o700)
        (prefix / 'bin').mkdir(); (prefix / 'bin/ruwac').symlink_to('../lib/ruwac/current/ruwac')
        receipt = release / 'installation.json'
        receipt.write_text(json.dumps({'version': baseline, 'artifacts': {'ruwac': hashlib.sha256(binary.read_bytes()).hexdigest()}})); receipt.chmod(0o600)
        old = {name: os.environ.get(name) for name in ['XDG_CONFIG_HOME', 'XDG_CACHE_HOME']}
        os.environ['XDG_CONFIG_HOME'] = str(base / 'config'); os.environ['XDG_CACHE_HOME'] = str(base / 'cache')
        generator = updates.update_units
        def accelerated(prefix):
            result = generator(prefix)
            # Only owned-fixture timing is accelerated; production remains six-hourly.
            result[updates.UNIT_NAMES[1]] = result[updates.UNIT_NAMES[1]].replace('OnActiveSec=5min', 'OnActiveSec=1s').replace('RandomizedDelaySec=5min', 'RandomizedDelaySec=0')
            return result
        updates.update_units = accelerated
        updates.scheduler_directory = lambda: manager_dir
        try:
            assert updates.automatic_install_setting(True, prefix)
            units = {name: (manager_dir / name).read_bytes() for name in updates.UNIT_NAMES}
            deadline = time.monotonic() + 60
            last = ''
            while time.monotonic() < deadline:
                state = subprocess.check_output(['systemctl', '--user', 'show', updates.UNIT_NAMES[0], '--property=Result', '--property=ExecMainStatus', '--property=ExecMainExitTimestampMonotonic'], text=True)
                values = dict(line.split('=', 1) for line in state.splitlines() if '=' in line)
                last = state
                if int(values.get('ExecMainExitTimestampMonotonic', '0')) > 0:
                    assert values.get('Result') == 'success' and values.get('ExecMainStatus') == '0', 'Owned automatic update helper failed'
                    break
                time.sleep(0.25)
            else:
                raise ValueError('Owned timer did not complete in the observation window')
            journal = subprocess.check_output(['journalctl', '--user', '-u', updates.UNIT_NAMES[0], '-o', 'cat', '--no-pager'], text=True)
            result = next(json.loads(line) for line in journal.splitlines() if line.startswith('{'))
            assert result['delivery'] == 'source-build' and result['available'] is False, 'Fixture requires current published baseline; do not compile a later release unexpectedly'
            assert updates.automatic_install_setting(False, prefix) is False
            active = subprocess.run(['systemctl', '--user', 'is-active', '--quiet', updates.UNIT_NAMES[1]]).returncode
            assert active != 0
            assert (root / 'current').readlink() == Path('releases/' + baseline)
            print(json.dumps({'status': 'PASS', 'real_systemd_timer_expiry': True, 'real_GitHub_release_commit_check': True, 'automatic_opt_out': True, 'installation_preserved': True, 'source_compilation': 'separate build gate; no newer version in this fixture'}))
        finally:
            subprocess.run(['systemctl', '--user', 'disable', '--now', updates.UNIT_NAMES[1]], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            subprocess.run(['systemctl', '--user', 'stop', updates.UNIT_NAMES[0]], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            for name, raw in units.items():
                path = manager_dir / name
                if path.is_file() and not path.is_symlink() and path.read_bytes() == raw:
                    path.unlink()
            subprocess.run(['systemctl', '--user', 'daemon-reload'], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            for name, value in old.items():
                if value is None: os.environ.pop(name, None)
                else: os.environ[name] = value


if __name__ == '__main__':
    main()
