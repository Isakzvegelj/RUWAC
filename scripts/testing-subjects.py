#!/usr/bin/env python3
"""Write exact artifact subjects for the existing release attestation action."""
import argparse
import hashlib
from pathlib import Path
import re


def subjects(bundles, rpms):
    archives = list(bundles.glob('*.tar.gz'))
    packages = list(rpms.glob('*.rpm'))
    if len(archives) != 1 or len(packages) != 2:
        raise ValueError('Expected one tested archive and both testing RPMs')
    matched = re.fullmatch(r'ruwac-((?:0|[1-9][0-9]*)\.[0-9]\.(?:[0-9]|[1-9][0-9]))-fedora44-x86_64-testing\.tar\.gz', archives[0].name)
    if not matched:
        raise ValueError('Unexpected testing archive name')
    version = matched.group(1)
    expected = {name + '-' + version + '-1.fc44.x86_64.rpm' for name in ['ruwac', 'ruwac-desktop']}
    if {p.name for p in packages} != expected:
        raise ValueError('Archive and RPM controller versions must match')
    result = []
    for path in sorted(archives + packages, key=lambda p: p.name):
        if path.is_symlink() or not path.is_file():
            raise ValueError('Artifact subjects must be regular files')
        result.append((path.name, hashlib.sha256(path.read_bytes()).hexdigest()))
    return result


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--bundles', type=Path, required=True)
    p.add_argument('--rpms', type=Path, required=True)
    p.add_argument('--output', type=Path, required=True)
    a = p.parse_args()
    records = subjects(a.bundles, a.rpms)
    # Names are relative public asset names, avoiding container path translation.
    with a.output.open('x') as stream:
        stream.write(''.join(digest + '  ' + name + '\n' for name, digest in records))
    print('Prepared', len(records), 'exact artifact attestation subjects')


if __name__ == '__main__':
    main()
