#!/usr/bin/env python3
"""Compare complete RPM bytes from separate assemblies of fixed compiled inputs."""
import argparse
import hashlib
import json
from pathlib import Path
import runpy
import stat
import subprocess
import tempfile
import time


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--build-root', type=Path, required=True)
    parser.add_argument('--evidence', type=Path, required=True)
    parser.add_argument('--rpms', type=Path, help='An existing first assembly to compare')
    parser.add_argument('--require-private', action='store_true')
    parser.add_argument('--require-tag', action='store_true')
    args = parser.parse_args()
    root = args.build_root.resolve()
    args.evidence.mkdir(parents=True, exist_ok=True)
    evidence = Path(tempfile.mkdtemp(prefix='rpm-reproducible-', dir=args.evidence))
    evidence.chmod(0o700)
    assemble = runpy.run_path(str(root / 'scripts/rpm-user.py'))['packages']
    binaries = {name: digest(root / 'target/release' / name) for name in ['ruwac', 'ruwac-desktop']}
    # Opposite inherited directory permissions must not affect package bytes.
    # Only this newly created fixture directory receives a default ACL.
    subprocess.run(['setfacl', '-k', str(evidence)], check=True)
    first = args.rpms.resolve() if args.rpms else evidence / 'first'
    if args.rpms is None:
        assemble(root, first, True, args.require_private, args.require_tag)
    time.sleep(2)  # Cross the whole-second RPM BuildTime boundary.
    subprocess.run(['setfacl', '-m', 'd:u::rwx,d:g::rwx,d:o::rx,d:m::rwx', str(evidence)], check=True)
    second = evidence / 'second-different-path'
    assemble(root, second, True, args.require_private, args.require_tag)
    left = {p.name: p for p in first.glob('*.rpm')}
    right = {p.name: p for p in second.glob('*.rpm')}
    assert len(left) == 2 and left.keys() == right.keys(), 'Split package set differs'
    epoch = int(subprocess.check_output(['git', '-C', str(root), 'show', '-s', '--format=%ct', 'HEAD'], text=True))
    packages = []
    for name in sorted(left):
        before, after = digest(left[name]), digest(right[name])
        assert before == after, 'Complete RPM bytes differ: ' + name
        for package in [left[name], right[name]]:
            value = subprocess.check_output(['rpm', '-qp', '--qf', '%{BUILDTIME}\n[%{FILEMTIMES}\n]', str(package)], stderr=subprocess.DEVNULL, text=True).splitlines()
            modes = subprocess.check_output(['rpm', '-qp', '--qf', '[%{FILEMODES}\\n]', str(package)],
                                            stderr=subprocess.DEVNULL, text=True).splitlines()
            assert all(stat.S_IMODE(int(mode)) == 0o755 for mode in modes if stat.S_ISDIR(int(mode))), 'Directory modes must be explicit 0755'
            assert int(value[0]) == epoch, 'BuildTime must equal the source commit time'
            assert all(int(item) <= epoch for item in value[1:]), 'Packaged file timestamp exceeds source time'
        packages.append({'name': name, 'sha256': before, 'complete_bytes_equal': True, 'directory_modes': '0755 independent of inherited default ACL'})
    assert binaries == {name: digest(root / 'target/release' / name) for name in binaries}
    receipt = {'PASS': True, 'source_epoch': epoch, 'compiled_inputs': binaries,
               'packages': packages, 'first': str(first), 'second': str(second),
               'scope': 'Repeated assembly from identical compiled inputs; independent source compilation is not claimed'}
    (evidence / 'acceptance.json').write_text(json.dumps(receipt, indent=2) + '\n')
    print('Deterministic complete RPM assembly PASS:', evidence)


if __name__ == '__main__':
    main()
