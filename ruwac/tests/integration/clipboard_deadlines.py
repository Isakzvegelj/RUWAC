#!/usr/bin/env python3
"""Real GDK cancellation on an explicitly owned Wayland fixture, not a physical seat."""
import argparse
import hashlib
import json
import os
import stat
import subprocess
import tempfile
from pathlib import Path


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--driver', type=Path, required=True)
    parser.add_argument('--evidence', type=Path, required=True)
    parser.add_argument('--owned-display', action='store_true')
    args = parser.parse_args()
    runtime = Path(os.environ.get('XDG_RUNTIME_DIR', '/nonexistent'))
    if (not args.owned_display or not os.environ.get('WAYLAND_DISPLAY', '').startswith('ruwac-')
            or not runtime.is_dir() or runtime.stat().st_uid != os.getuid()
            or stat.S_IMODE(runtime.stat().st_mode) != 0o700):
        parser.error('Use an owned isolated Wayland fixture; never run this provider on the owner desktop')
    args.evidence.mkdir(parents=True, exist_ok=True)
    results = []
    before = hashlib.sha256(args.driver.read_bytes()).hexdigest()
    for reader in ('text', 'png', 'rdp'):
        for mode in ('focus', 'deadline'):
            root = Path(tempfile.mkdtemp(prefix='clipboard-deadline-', dir=args.evidence))
            root.chmod(0o700)
            completed = subprocess.run([str(args.driver), reader, mode, str(root)],
                                       capture_output=True, text=True, timeout=12,
                                       env={**os.environ, 'GTK_A11Y': 'none', 'GSETTINGS_BACKEND': 'memory'})
            (root / 'stdout.log').write_text(completed.stdout)
            (root / 'stderr.log').write_text(completed.stderr)
            detail = json.loads(completed.stdout.splitlines()[-1]) if completed.stdout.splitlines() else {}
            result = {'id': f'GDK-STALLED-{reader.upper()}-{mode.upper()}',
                      'status': 'PASS' if completed.returncode == 0 else 'FAIL', 'detail': detail}
            results.append(result)
            print(result['id'], result['status'], flush=True)
    assert hashlib.sha256(args.driver.read_bytes()).hexdigest() == before
    report = {'results': results, 'driver_sha256': before,
              'scope': 'Actual GDK and GIO on owned Wayland; same-process deliberately stalled provider',
              'limitations': ['Not a physical-input result', 'Not external Windows/Fedora interoperability']}
    (args.evidence / 'clipboard-deadlines.json').write_text(json.dumps(report, indent=2) + '\n')
    return 0 if all(row['status'] == 'PASS' for row in results) else 1


if __name__ == '__main__':
    raise SystemExit(main())
