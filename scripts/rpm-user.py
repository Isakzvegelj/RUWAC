#!/usr/bin/env python3
"""Package existing controller-versioned Fedora testing binaries; never release them."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import runpy
import shutil
import subprocess
import tempfile
import tomllib


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def packages(root, output, desktop=True, require_private=False, require_tag=False):
    root = root.resolve()
    output = output.resolve()
    if platform.machine() != 'x86_64':
        raise ValueError('Testing RPMs currently require x86_64')
    os_release = Path('/etc/os-release').read_text()
    if 'ID=fedora\n' not in os_release or 'VERSION_ID=44\n' not in os_release:
        raise ValueError('Testing RPMs must be assembled on Fedora 44')
    if not shutil.which('rpmbuild'):
        raise ValueError('Install the Fedora rpm-build tool')
    version = tomllib.loads((root / 'ruwac/Cargo.toml').read_text())['workspace']['package']['version']
    if not re.fullmatch(r'(0|[1-9][0-9]*)\.[0-9]\.([0-9]|[1-9][0-9])', version):
        raise ValueError('A controller-assigned numeric version is required')
    applications = ['ruwac', 'ruwac-desktop'] if desktop else ['ruwac']
    for name in applications:
        actual = subprocess.check_output([str(root / 'target/release' / name), '--version'], text=True).strip()
        expected = ('RUWAC desktop ' if name.endswith('-desktop') else 'RUWAC ') + version
        if actual != expected:
            raise ValueError('Compiled artifact and controller version differ')
    commit = subprocess.check_output(['git', '-C', str(root), 'rev-parse', 'HEAD'], text=True).strip()
    tagged = subprocess.run(['git', '-C', str(root), 'rev-parse', 'v' + version + '^{commit}'], capture_output=True, text=True)
    clean = not subprocess.check_output(['git', '-C', str(root), 'diff', '--name-only', 'HEAD'], text=True).strip()
    source_commit = commit if clean and tagged.returncode == 0 and tagged.stdout.strip() == commit else None
    if require_tag and source_commit is None:
        raise ValueError('Publication RPMs require the clean exact controller tag')
    epoch = int(subprocess.check_output(['git', '-C', str(root), 'show', '-s', '--format=%ct', commit], text=True))
    output.mkdir(parents=True, exist_ok=True)
    names = ['ruwac-' + version + '-1.fc44.x86_64.rpm']
    if desktop:
        names += ['ruwac-desktop-' + version + '-1.fc44.x86_64.rpm']
    if any((output / name).exists() or (output / name).is_symlink() for name in names):
        raise FileExistsError('Preserve existing RPM outputs')
    helpers = runpy.run_path(str(Path(__file__).with_name('bundle-user.py')))
    scanner = runpy.run_path(str(root / 'scripts/privacy-check.py'))
    terms = scanner['private_terms'](root, required=require_private)
    result = {'schema': 1, 'kind': 'fedora44-testing-rpms', 'version': version,
              'source_commit': source_commit, 'security_qualified': False,
              'signature': 'unsigned; verify exact GitHub provenance before installation',
              'packages': []}
    # Packaging work and outputs stay in the selected external build workspace.
    with tempfile.TemporaryDirectory(prefix='.rpm-build-', dir=output) as temporary:
        top = Path(temporary)
        for name in ['BUILD', 'BUILDROOT', 'RPMS', 'SOURCES', 'SPECS', 'SRPMS']:
            (top / name).mkdir()
        payload = top / 'ruwac-rpm-payload'
        (payload / 'target/release').mkdir(parents=True)
        for name in applications:
            shutil.copy2(root / 'target/release' / name, payload / 'target/release' / name)
        shutil.copy2(root / 'LICENSE', payload / 'LICENSE')
        (payload / 'TESTING.txt').write_text(
            'RUWAC upstream Fedora 44 testing candidate\n'
            'Installing, updating or removing these packages does not restart daemons or windows, '
            'alter user profiles/credentials, or install per-user update timers.\n'
            'Desktop RUSTSEC-2023-0071 remains unresolved. These are unsigned testing RPMs, '
            'not Fedora repository or complete security qualification.\n'
            'The version is assigned by the existing source release controller. The fixed RPM '
            'packaging revision is 1.fc44.\n')
        components = [('core', 'ruwac-cli')]
        if desktop:
            components.append(('desktop', 'ruwac-desktop'))
            shutil.copy2(Path(__file__).parents[1] / 'ruwac/packaging/ruwac.desktop', payload / 'ruwac.desktop')
        licenses = {}
        for component, application in components:
            rows = helpers['notices'](root, payload / 'notices' / component,
                                      helpers['runtime_packages'](root, {application}))
            declarations = sorted({row['declared_license'] for row in rows})
            if any(not isinstance(item, str) or not item or '\n' in item or '%' in item for item in declarations):
                raise ValueError('Invalid dependency license metadata')
            licenses[component] = ' AND '.join('(' + item + ')' for item in declarations)
        receipt = {'kind': 'fedora-testing-rpm', 'version': version, 'source_commit': source_commit,
                   'artifacts': {name: digest(root / 'target/release' / name) for name in applications}}
        (payload / 'source.json').write_text(json.dumps(receipt, sort_keys=True) + '\n')
        with (payload / 'TESTING.txt').open('a') as stream:
            stream.write('Source/artifact receipt: ' + json.dumps(receipt, sort_keys=True) + '\n')
        helpers['scan'](root, payload, require_private, applications)
        helpers['archive'](payload, top / 'SOURCES/ruwac-testing-payload.tar.gz', epoch)
        spec = top / 'SPECS/ruwac.spec'
        shutil.copy2(Path(__file__).parents[1] / 'ruwac/packaging/ruwac.spec', spec)
        command = ['rpmbuild', '-bb', str(spec), '--define', '_topdir ' + str(top),
                   '--define', '_buildhost build.invalid', '--define', '_build_id_links none',
                   '--define', 'dist .fc44', '--define', 'ruwac_version ' + version,
                   '--define', 'ruwac_core_license ' + licenses['core']]
        if desktop:
            command += ['--define', 'ruwac_desktop_license ' + licenses['desktop']]
        else:
            command += ['--without', 'desktop']
        with (top / 'rpmbuild.log').open('w') as log:
            build = subprocess.run(command, env={**os.environ, 'SOURCE_DATE_EPOCH': str(epoch)}, stdout=log, stderr=subprocess.STDOUT)
        if build.returncode:
            # This external diagnostic is private and never a release asset.
            shutil.copy2(top / 'rpmbuild.log', output / 'rpmbuild-failure.log')
            raise ValueError('RPM build failed; inspect external private diagnostic')
        for name in names:
            rpm = top / 'RPMS/x86_64' / name
            query = subprocess.check_output(['rpm', '-qp', '--qf', '%{NAME}\n%{VERSION}\n%{RELEASE}\n%{ARCH}\n%{BUILDHOST}\n%{LICENSE}\n', str(rpm)], stderr=subprocess.DEVNULL)
            if scanner['findings'](query, terms):
                raise ValueError('RPM header privacy check failed')
            metadata = query.decode().splitlines()
            if metadata[1:5] != [version, '1.fc44', 'x86_64', 'build.invalid']:
                raise ValueError('RPM metadata differs from controller/platform')
            requires = subprocess.check_output(['rpm', '-qp', '--requires', str(rpm)], stderr=subprocess.DEVNULL, text=True).splitlines()
            if metadata[0] == 'ruwac' and any(re.search(r'gtk|adwaita|gdk|rdp|pango|cairo', x, re.I) for x in requires):
                raise ValueError('Headless RPM gained a graphical dependency')
            shutil.copy2(rpm, output / name)
            checksum = digest(output / name)
            (output / (name + '.sha256')).write_text(checksum + '  ' + name + '\n')
            result['packages'].append({'name': name, 'sha256': checksum, 'requires': requires, 'license': metadata[5]})
    return result


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--build-root', type=Path, required=True)
    p.add_argument('--output', type=Path, required=True)
    p.add_argument('--headless-only', action='store_true')
    p.add_argument('--require-private', action='store_true')
    p.add_argument('--require-tag', action='store_true')
    a = p.parse_args()
    try:
        print(json.dumps(packages(a.build_root, a.output, not a.headless_only,
                                  a.require_private, a.require_tag), indent=2))
    except (OSError, ValueError, KeyError, subprocess.CalledProcessError):
        print('Testing RPM assembly failed; preserve external diagnostic evidence', file=__import__('sys').stderr)
        return 1
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
