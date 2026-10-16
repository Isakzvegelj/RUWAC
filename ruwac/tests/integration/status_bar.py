#!/usr/bin/env python3
"""Actual PTY clicks on clipped Unicode labels and hidden-window arrows."""
import argparse
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import tempfile
import unicodedata

from spike import Client, Tui, eventually


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--daemon', type=Path)
    parser.add_argument('--evidence', type=Path, required=True)
    args = parser.parse_args()
    args.evidence.mkdir(parents=True, exist_ok=True)
    root = Path(tempfile.mkdtemp(prefix='status-bar-', dir=args.evidence)).resolve()
    root.chmod(0o700)
    sock = root / 'control.sock'
    env = {**os.environ, 'TERM': 'xterm-256color', 'RUWAC_STATE_FILE': str(root / 'state.json')}
    for key in ('HOME', 'XDG_CONFIG_HOME', 'XDG_DATA_HOME', 'XDG_CACHE_HOME'):
        path = root / key.lower()
        path.mkdir(mode=0o700)
        env[key] = str(path)
    receiver = root / 'receiver.py'
    receiver.write_text("import os,sys,tty\nfrom pathlib import Path\ntty.setraw(0)\n"
                        "p=Path(sys.argv[1]);p.write_bytes(b'')\n"
                        "os.write(1,b'GENERATED_STATUS_READY\\x1b[?1000h\\x1b[?1006h')\n"
                        "while True:\n b=os.read(0,4096)\n if not b:break\n"
                        " with p.open('ab')as f:f.write(b)\n")
    log = (root / 'daemon.log').open('wb')
    daemon = subprocess.Popen([str((args.daemon or args.binary).resolve()), '--socket', str(sock), 'serve'],
                              env=env, stdout=log, stderr=log)
    view = None
    checks = []

    def rpc(method, params=None):
        with Client(sock) as c:
            return c.call(method, params)

    def width(text):
        return sum(0 if unicodedata.combining(c) else 2 if unicodedata.east_asian_width(c) in 'WF' else 1 for c in text)

    def bar(offset=0):
        matches = re.findall(rb'\x1b\[24;1H\x1b\[0;38;2;16;32;48;48;2;173;216;230m([^\x1b]*)\x1b\[0m', view.data[offset:])
        return matches[-1].decode() if matches else None

    def title(index, offset=0):
        return f'\x1b]2;RUWAC generated · {index}:'.encode() in view.data[offset:]

    def click(column, expected):
        offset = len(view.data)
        view.send(f'\x1b[<0;{column+1};24M\x1b[<0;{column+1};24m'.encode())
        def selected():
            found = re.findall(rb'\x1b\]2;RUWAC generated \xc2\xb7 (\d+):', view.data[offset:])
            return int(found[-1])+1 if found else None
        index = eventually(selected, 15)-1
        if expected is not None:
            assert index == expected, (index, expected)
        eventually(lambda: bar(offset), 15)
        return index

    def column_for(text, token):
        segment = text.rsplit(' · ', 1)[-1]
        at = len(text) - len(segment) + segment.index(token)
        return width(text[:at])

    try:
        eventually(lambda: sock.exists())
        row = rpc('session.create', {'name': 'generated', 'argv': [sys.executable, str(receiver), str(root / 'input-0')]})
        windows = [row['window']]
        for n in range(18):
            if n:
                created = rpc('window.create', {'session': row['session'], 'argv': [sys.executable, str(receiver), str(root / f'input-{n}')]})
                windows.append(created['window'])
            rpc('window.rename', {'window': windows[n], 'name': '界é-long-window-title-' * 12})
        eventually(lambda: all((root / f'input-{n}').exists() for n in range(18)))
        view = Tui(args.binary.resolve(), sock, row['session'], root / 'client.pty', env=env)
        eventually(lambda: bar())
        view.send(b'\x010')
        eventually(lambda: title(0))
        line = eventually(lambda: (s if '0:' in s.rsplit(' · ', 1)[-1] else None) if (s := bar()) else None)
        assert width(line) == 80 and '…' in line
        initial = {n: (root / f'input-{n}').read_bytes() for n in range(18)}
        segment = line.rsplit(' · ', 1)[-1]
        neighbor = next(int(n) for n in re.findall(r'(\d+):', segment) if int(n) != 0)
        click(column_for(line, str(neighbor) + ':'), neighbor)
        checks.append('clipped-Unicode-window-label-selects-stable-target')

        for _ in range(30):
            line = bar()
            segment = line.rsplit(' · ', 1)[-1]
            indices = list(map(int, re.findall(r'(\d+):', segment)))
            if 17 in indices:
                click(column_for(line, '17:'), 17)
                break
            if '›' not in segment:
                # The final visible label can be clipped before its colon.
                # With no right-hidden indicator, its last visible cell must
                # still select the known final window, even when it reads 17….
                click(width(line.rstrip())-1, 17)
                break
            selected = click(column_for(line, '›'), None)
            # One final label may be too narrow even to show its index. The
            # arrow must advance beyond every completely visible label.
            assert max(indices) < selected <= min(17, max(indices)+2), (segment, selected)
        else:
            raise AssertionError('Hidden-window arrow did not reach the last window')
        eventually(lambda: title(17))
        checks.append('hidden-window-arrows-reveal-and-select-far-window')

        # These are real mouse-reporting application PTYs. Prove their receipt
        # path works before checking that status clicks never reach a child.
        before = (root / 'input-17').read_bytes()
        remote = b'\x1b[<0;2;2M\x1b[<0;2;2m'
        view.send(remote)
        eventually(lambda: (root / 'input-17').read_bytes() == before + remote)
        initial[17] += remote
        barrier=b'INPUT_BARRIER'
        view.send(b'\x1b[<0;1;24M\x1b[<0;1;24m'+barrier)
        eventually(lambda: (root/'input-17').read_bytes().endswith(barrier))
        initial[17]+=barrier
        for n in range(18):
            assert (root / f'input-{n}').read_bytes() == initial[n], n
        assert daemon.poll() is None
        checks.append('bottom-row-label-arrow-and-prefix-clicks-never-leak-to-applications')
        (root / 'acceptance.json').write_text(json.dumps({'status': 'PASS', 'checks': checks,
            'scope': 'Actual owned PTY SGR events and mouse-reporting child processes; generated names only, no physical mouse claim'}, indent=2) + '\n')
        print('STATUS_BAR PASS', len(checks), 'groups', flush=True)
    finally:
        if view:
            view.finish('term')
        if daemon.poll() is None:
            daemon.terminate()
            try:
                daemon.wait(timeout=10)
            except subprocess.TimeoutExpired:
                daemon.kill()
                daemon.wait()
        log.close()


if __name__ == '__main__':
    main()
