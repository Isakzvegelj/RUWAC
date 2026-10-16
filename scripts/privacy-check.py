#!/usr/bin/env python3
"""Payload-free publication checks. Site policy is supplied privately, outside Git."""
import argparse
import ipaddress
import json
import os
from pathlib import Path
import re
import subprocess
import sys

# Loopback and RFC 5737 examples are portable; private-network literals are not.
IPV4 = re.compile(r'(?<![\w.])(?:\d{1,3}\.){3}\d{1,3}(?![\w.])')
RULES = {
    'private-key': re.compile(r'-----BEGIN (?:[A-Z0-9 ]+ )?PRIVATE KEY-----'),
    'personal-path': re.compile(r'/(?:home|Users)/[A-Za-z0-9_.-]+|/run/user/[1-9][0-9]*'),
    'credential-url': re.compile(r'(?:https?|ssh|sftp)://[^\s/:]+:[^\s/@]+@'),
    'access-token': re.compile(r'\b(?:gh[pousr]_[A-Za-z0-9]{30,}|github_pat_[A-Za-z0-9_]{40,}|AKIA[A-Z0-9]{16})\b'),
}
PRIVATE_NETS = tuple(ipaddress.ip_network((value, bits)) for value, bits in ((10 << 24, 8), ((172 << 24) | (16 << 16), 12), ((192 << 24) | (168 << 16), 16)))
DOCUMENTATION_NETS = tuple(ipaddress.ip_network(x) for x in ('192.0.2.0/24', '198.51.100.0/24', '203.0.113.0/24'))


class PolicyTerms(list):
    def __init__(self, terms, public_attributions=()):
        super().__init__(terms)
        self.public_attributions = tuple(public_attributions)


def private_terms(root, filename=None, required=False):
    filename = filename or os.environ.get('RUWAC_PRIVATE_DENYLIST')
    if not filename:
        if required:
            raise ValueError('Private publication policy is required; provide an external policy file')
        return []
    path = Path(filename).resolve(strict=True)
    if path == root or root in path.parents:
        raise ValueError('Private publication policy must be outside the repository')
    if path.stat().st_mode & 0o077:
        raise ValueError('Private publication policy must have owner-only permissions')
    data = json.loads(path.read_text())
    terms = data['terms']
    if not isinstance(terms, list) or not terms or any(not isinstance(x, str) or len(x) < 3 for x in terms):
        raise ValueError('Invalid private publication policy')
    public = data.get('public_attributions', [])
    if not isinstance(public, list) or any(not isinstance(x, str) or len(x) < 6 or any(c in x for c in '\r\n\x00') for x in public):
        raise ValueError('Invalid public attribution policy')
    return PolicyTerms(terms, public)


def findings(data, terms=()):
    """Return only rule identifiers and line numbers, never matching values."""
    text = data.decode('utf-8', errors='replace') if isinstance(data, bytes) else data
    result = set()
    for number, line in enumerate(text.splitlines(), 1):
        for rule, pattern in RULES.items():
            if pattern.search(line):
                result.add((rule, number))
        for match in IPV4.finditer(line):
            try:
                address = ipaddress.ip_address(match.group())
            except ValueError:
                continue
            if address.is_loopback or any(address in net for net in DOCUMENTATION_NETS):
                continue
            if any(address in network for network in PRIVATE_NETS):
                result.add(('private-network', number))
        policy_line = line
        for attribution in getattr(terms, 'public_attributions', ()):
            policy_line = re.sub(r'(?<![A-Za-z0-9])' + re.escape(attribution) + r'(?![A-Za-z0-9])', '', policy_line)
        for term in terms:
            # Paths and address prefixes are literal substrings; short host/account
            # terms require boundaries, including within underscored fixture names.
            pattern = re.escape(term) if '/' in term or term.endswith('.') else r'(?<![A-Za-z0-9])' + re.escape(term) + r'(?![A-Za-z0-9])'
            if re.search(pattern, policy_line, re.IGNORECASE):
                result.add(('private-policy', number))
    return sorted(result)


def verify(root, paths, terms=()):
    failures = []
    for name in paths:
        path = root / name
        name_findings = findings(name, terms)
        label = name if not name_findings else '[redacted source path]'
        if name_findings:
            failures.append('Source path failed privacy rules: ' + label)
        if path.is_symlink() or not path.is_file():
            failures.append(f'{label}: invalid source file')
            continue
        for rule, number in findings(path.read_bytes(), terms):
            failures.append(f'{label}:{number}: {rule}')
    if failures:
        raise ValueError('\n'.join(failures))
    return len(paths)


def verify_outgoing(root, base, terms=()):
    if not re.fullmatch(r'[0-9a-fA-F]{40,64}', base or '') or set(base) == {'0'}:
        # A newly created branch is checked in full; no silent shallow-history fallback.
        revisions = ['HEAD']
    else:
        subprocess.run(['git', '-C', str(root), 'cat-file', '-e', base + '^{commit}'], check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        revisions = [base + '..HEAD']
    commits = subprocess.check_output(['git', '-C', str(root), 'rev-list', *revisions]).decode().split()
    for commit in commits:
        message = subprocess.check_output(['git', '-C', str(root), 'show', '-s', '--format=%B', commit])
        if findings(message, terms):
            raise ValueError('Outgoing commit metadata failed privacy rules: ' + commit)
        entries = subprocess.check_output(['git', '-C', str(root), 'ls-tree', '-rz', commit]).split(b'\0')
        for entry in filter(None, entries):
            meta, name = entry.split(b'\t', 1)
            kind, sha = meta.split()[1:]
            if kind != b'blob':
                raise ValueError('Outgoing tree contains a non-file entry')
            data = subprocess.check_output(['git', '-C', str(root), 'cat-file', 'blob', sha.decode()])
            if findings(data, terms) or findings(name.decode(), terms):
                # No match contents or credential values enter diagnostics.
                raise ValueError('Outgoing commit snapshot failed privacy rules: ' + commit + ' ' + ('[redacted source path]' if findings(name.decode(), terms) else name.decode()))
    return len(commits)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument('--denylist', type=Path)
    parser.add_argument('--require-private', action='store_true')
    parser.add_argument('--stdin', action='store_true', help='Scan metadata without echoing it')
    parser.add_argument('--tracked', action='store_true')
    parser.add_argument('--inventory', action='store_true', help='Scan the explicit publication inventory in a source-only directory')
    parser.add_argument('--outgoing', help='Audit every new commit snapshot and message since the base SHA')
    args = parser.parse_args()
    root = args.root.resolve()
    try:
        terms = private_terms(root, args.denylist, args.require_private)
        if args.outgoing is not None:
            count = verify_outgoing(root, args.outgoing, terms)
            print(f'Outgoing privacy check PASS: {count} commits')
        elif args.stdin:
            found = findings(sys.stdin.buffer.read(), terms)
            if found:
                raise ValueError('\n'.join(f'metadata:{line}: {rule}' for rule, line in found))
            print('Privacy metadata check PASS')
        else:
            if args.inventory:
                import runpy
                paths = [str(x) for x in runpy.run_path(str(root / 'scripts/public-files.py'))['files'](root)]
            else:
                paths = subprocess.check_output(['git', '-C', str(root), 'ls-files', '-z']).decode().split('\0')
            count = verify(root, list(filter(None, paths)), terms)
            print(f'Privacy check PASS: {count} tracked files; private policy {"enabled" if terms else "not supplied (public rules only)"}')
    except (ValueError, OSError, KeyError, json.JSONDecodeError, subprocess.CalledProcessError) as error:
        # Parse/IO errors can contain a private filename; never emit their text.
        if isinstance(error, ValueError) and not isinstance(error, json.JSONDecodeError):
            print(str(error), file=sys.stderr)
        else:
            print('Privacy policy or source could not be read', file=sys.stderr)
        return 1
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
