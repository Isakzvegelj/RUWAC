#!/usr/bin/env python3
"""Actual upstream testing RPM payload/dependency checks and owned transactions."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile

from spike import Client, eventually


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--rpms', type=Path, required=True)
    p.add_argument('--previous-rpms', type=Path)
    p.add_argument('--evidence', type=Path, required=True)
    p.add_argument('--query-only', action='store_true')
    p.add_argument('--run-owned-transaction', action='store_true')
    a = p.parse_args()
    a.evidence.mkdir(parents=True, exist_ok=True)
    root = Path(tempfile.mkdtemp(prefix='rpm-transaction-', dir=a.evidence)).resolve()
    root.chmod(0o700)
    packages = sorted(a.rpms.glob('*.rpm'))
    assert len(packages) == 2, 'Both headless and desktop testing RPMs are required'
    checks = []
    unpacked = root / 'unpacked'
    unpacked.mkdir()
    for rpm in packages:
        subprocess.run(['rpm', '-K', str(rpm)], capture_output=True, check=True)
        values = subprocess.check_output(['rpm', '-qp', '--qf', '%{NAME}\n%{VERSION}\n%{RELEASE}\n%{ARCH}\n%{BUILDHOST}\n', str(rpm)], stderr=subprocess.DEVNULL, text=True).splitlines()
        name, version, release, arch, host = values
        assert name in ['ruwac', 'ruwac-desktop'] and re.fullmatch(r'\d+\.\d+\.\d+', version)
        assert release == '1.fc44' and arch == 'x86_64' and host == 'build.invalid'
        paths = subprocess.check_output(['rpm', '-qpl', str(rpm)], stderr=subprocess.DEVNULL, text=True).splitlines()
        assert paths and all(path.startswith('/usr/') and '..' not in Path(path).parts for path in paths)
        assert not subprocess.check_output(['rpm', '-qp', '--scripts', str(rpm)], stderr=subprocess.DEVNULL).strip(), 'Packages must not restart sessions or change user configuration'
        requires = subprocess.check_output(['rpm', '-qp', '--requires', str(rpm)], stderr=subprocess.DEVNULL, text=True).splitlines()
        if name == 'ruwac':
            assert not any(re.search(r'gtk|gdk|adwaita|rdp|pango|cairo', item, re.I) for item in requires)
        else:
            assert 'ruwac = ' + version + '-' + release in requires
            assert any('libgtk-4.so' in item for item in requires)
            assert '/usr/share/applications/si.z4.ruwac.desktop' in paths
        source = subprocess.Popen(['rpm2cpio', str(rpm)], stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        result = subprocess.run(['cpio', '-idm', '--quiet', '--no-absolute-filenames'], cwd=unpacked, stdin=source.stdout, capture_output=True)
        source.stdout.close()
        assert source.wait(timeout=10) == 0 and result.returncode == 0
        checks.append(name + '-payload-header-dependencies-and-no-scripts')
    core = unpacked / 'usr/bin/ruwac'
    desktop = unpacked / 'usr/bin/ruwac-desktop'
    assert subprocess.check_output([str(core), '--version'], text=True).strip() == 'RUWAC ' + version
    assert subprocess.check_output([str(desktop), '--version'], text=True).strip() == 'RUWAC desktop ' + version
    subprocess.run(['desktop-file-validate', str(unpacked / 'usr/share/applications/si.z4.ruwac.desktop')], capture_output=True, check=True)
    for component in ['ruwac', 'ruwac-desktop']:
        license_dir = unpacked / 'usr/share/licenses' / component
        assert (license_dir / 'LICENSE').is_file()
        indexes = list(license_dir.rglob('index.json'))
        assert len(indexes) == 1 and json.loads(indexes[0].read_text())['packages']
    checks.append('actual-packaged-executables-launcher-and-notices')
    marker = unpacked / 'usr/share/ruwac/rpm-installation.json'
    if marker.exists():
        assert json.loads(marker.read_text())['kind'] == 'fedora-testing-rpm'
        env = {**os.environ, 'HOME': str(root / 'update-home'),
               'XDG_CONFIG_HOME': str(root / 'update-config'),
               'XDG_CACHE_HOME': str(root / 'update-cache')}
        for mode in [['--install'], ['--rollback'], ['--auto-install', 'on'], ['--auto-check', 'on'], ['--testing-bundles', 'on']]:
            refusal = subprocess.run([str(core), 'update', *mode], env=env, capture_output=True, text=True)
            assert refusal.returncode != 0 and 'RPM installation is managed by DNF' in refusal.stdout
        for mode in [['--auto-check', 'status'], ['--testing-bundles', 'status'], ['--install-auto'], ['--check-auto']]:
            response = subprocess.run([str(core), 'update', *mode], env=env, capture_output=True, text=True)
            assert response.returncode == 0 and json.loads(response.stdout)['distribution'] == 'rpm'
        assert not Path(env['HOME']).exists() and not Path(env['XDG_CONFIG_HOME']).exists() and not Path(env['XDG_CACHE_HOME']).exists()
        # A process from an earlier package must remain managed after an upgrade.
        changed = json.loads(marker.read_text()); changed['version'] = 'generated-newer-stamp'
        marker.write_text(json.dumps(changed))
        # Extracted payloads cannot authorize a root helper. Its canonical system
        # path is deliberately absent here; real installed-root status/enable and
        # DNF transactions are covered by the separate owned OS fixture.
        if (unpacked / 'usr/share/ruwac/rpm-update.py').exists():
            assert not Path('/usr/bin/ruwac-rpm-update').exists(), 'Use an isolated host without an installed system updater for extracted-payload checks'
            response = subprocess.run([str(core), 'update', '--auto-install', 'status'], env=env, capture_output=True, text=True)
            assert response.returncode != 0 and 'RPM installation is managed by DNF' in response.stdout
            assert (unpacked / 'usr/lib/systemd/system/ruwac-rpm-update.timer').is_file()
            assert (unpacked / 'usr/lib/systemd/system/ruwac-rpm-update.service').is_file()
            checks.append('extracted-payload-cannot-substitute-for-canonical-root-updater')
        else:
            response = subprocess.run([str(core), 'update', '--auto-install', 'status'], env=env, capture_output=True, text=True)
            assert response.returncode == 0 and json.loads(response.stdout)['distribution'] == 'rpm'
        checks.append('rpm-updater-refuses-shadow-install-and-preserves-private-settings')
    if a.run_owned_transaction:
        assert not a.query_only and a.previous_rpms, 'Real upgrade requires genuine prior packages'
        previous = sorted(a.previous_rpms.glob('*.rpm'))
        assert len(previous) == 2
        install_root = root / 'installroot'
        install_root.mkdir(mode=0o700)
        assert install_root.is_relative_to(root) and install_root != Path('/')
        prefix = [] if os.geteuid() == 0 else ['sudo', '-n']
        base = prefix + ['rpm', '--root', str(install_root), '--dbpath', '/usr/lib/sysimage/rpm']
        subprocess.run(base + ['--initdb'], capture_output=True, check=True)
        # Private database has no copied system dependency records. Runtime uses
        # the real Fedora host libraries; this does not prove DNF resolution in
        # a clean OS or a signed repository transaction.
        subprocess.run(base + ['-ivh', '--nodeps', '--noscripts', *map(str, previous)], capture_output=True, check=True)
        home = root / 'private-home'
        home.mkdir(mode=0o700)
        marker = home / 'private-profile.json'
        marker.write_text('{"generated":"private configuration sentinel"}\n')
        marker.chmod(0o600)
        before = digest(marker)
        env = {**os.environ, 'HOME': str(home), 'XDG_CONFIG_HOME': str(home / 'config'),
               'XDG_DATA_HOME': str(home / 'data'), 'XDG_CACHE_HOME': str(home / 'cache')}
        for key in ['XDG_CONFIG_HOME', 'XDG_DATA_HOME', 'XDG_CACHE_HOME']:
            Path(env[key]).mkdir(mode=0o700)
        binary = install_root / 'usr/bin/ruwac'
        socket = root / 'control.sock'
        with (root / 'daemon.log').open('w') as log:
            daemon = subprocess.Popen([str(binary), '--socket', str(socket), 'serve'], env=env, stdout=log, stderr=subprocess.STDOUT)
            try:
                eventually(socket.exists)
                with Client(socket) as client:
                    receiver = Path(__file__).resolve().parents[1] / 'fixtures/receiver.py'
                    session = client.call('session.create', {'name': 'Generated RPM lifecycle', 'argv': [sys.executable, str(receiver), str(root / 'received.bin')]})
                    client.call('pane.wait', {'pane': session['pane'], 'contains': 'READY', 'timeout_ms': 5000})
                    subprocess.run(base + ['-Uvh', '--nodeps', '--noscripts', *map(str, packages)], capture_output=True, check=True)
                    assert subprocess.check_output([str(binary), '--version'], text=True).strip() == 'RUWAC ' + version
                    client.call('pane.sendText', {'pane': session['pane'], 'text': 'AFTER_RPM_UPGRADE'})
                    eventually(lambda: (root / 'received.bin').read_bytes() == b'AFTER_RPM_UPGRADE')
                    os.kill(session['pid'], 0)
                    assert daemon.poll() is None and digest(marker) == before
                    checks.append('real-private-rpm-upgrade-preserves-running-daemon-child-and-config')
                    subprocess.run(base + ['-e', '--nodeps', '--noscripts', 'ruwac-desktop', 'ruwac'], capture_output=True, check=True)
                    assert not binary.exists() and digest(marker) == before
                    client.call('pane.sendText', {'pane': session['pane'], 'text': '_AFTER_REMOVE'})
                    eventually(lambda: (root / 'received.bin').read_bytes() == b'AFTER_RPM_UPGRADE_AFTER_REMOVE')
                    checks.append('real-private-rpm-removal-preserves-running-daemon-child-and-config')
                    client.call('daemon.stop')
            finally:
                if daemon.poll() is None:
                    daemon.terminate()
                daemon.wait(timeout=8)
    result = {'status': 'PASS', 'checks': checks, 'rpms': {pkg.name: digest(pkg) for pkg in packages},
              'scope': 'Actual Fedora RPM payloads and executable versions; optional real private RPM database transactions use --nodeps and host runtime libraries. No clean-OS dependency, signing or physical desktop qualification.',
              'security_qualified': False}
    (root / 'acceptance.json').write_text(json.dumps(result, indent=2) + '\n')
    print('RPM acceptance PASS', len(checks), 'groups; private evidence', root)


if __name__ == '__main__':
    main()
