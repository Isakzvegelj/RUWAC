#!/usr/bin/env python3
"""Native GTK/Wayland allocation, complete large canvas and repeated resizing."""
import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time

from spike import Client, eventually


def main():
    p = argparse.ArgumentParser()
    p.add_argument('--binary', type=Path, required=True)
    p.add_argument('--gui', type=Path, required=True)
    p.add_argument('--evidence', type=Path, required=True)
    a = p.parse_args()
    if not os.environ.get('WAYLAND_DISPLAY', '').startswith('ruwac-'):
        p.error('Explicit owned Wayland compositor required')
    root = Path(tempfile.mkdtemp(prefix='native-natural-', dir=a.evidence)).resolve()
    root.chmod(0o700)
    runtime = Path(tempfile.mkdtemp(prefix='ruwac-native-grid-', dir='/tmp'))
    runtime.chmod(0o700)
    env = {**os.environ, 'GTK_A11Y': 'none', 'GSETTINGS_BACKEND': 'memory', 'RUWAC_STATE_FILE': str(root / 'state.json')}
    for k in ('HOME', 'XDG_CONFIG_HOME', 'XDG_DATA_HOME', 'XDG_CACHE_HOME'):
        q = root / k.lower()
        q.mkdir(mode=0o700)
        env[k] = str(q)
    fixture = root / 'fixture'
    fixture.mkdir(mode=0o700)
    app = root / 'paint.py'
    app.write_text("import fcntl,json,os,signal,struct,sys,tty\nfrom pathlib import Path\n"
                   "p=Path(sys.argv[1]);tty.setraw(0)\n"
                   "def report(*_):\n r,c=struct.unpack('HHHH',fcntl.ioctl(0,0x5413,b'\\0'*8))[:2]\n"
                   " q=p.with_suffix('.tmp');q.write_text(json.dumps({'rows':r,'cols':c,'pid':os.getpid()}));q.replace(p)\n"
                   "signal.signal(signal.SIGWINCH,report);report()\n"
                   "os.write(1,b'GENERATED_NATIVE_GRID_READY')\n"
                   "while True:\n data=os.read(0,4096)\n if not data:break\n"
                   " if b'p'in data:os.write(1,b'\\x1b[48;2;20;42;70m\\x1b[2J\\x1b[H\\x1b[?25l')\n")
    children, checks, timings = [], [], []
    seq = 0
    sock = runtime / 'control.sock'

    def start(command, name):
        log = (root / (name + '.log')).open('wb')
        child = subprocess.Popen(command, env=env, stdout=log, stderr=log)
        children.append((child, log))
        return child

    def rpc(method, params=None):
        with Client(sock) as c:
            return c.call(method, params)

    def command(action, **kw):
        nonlocal seq
        seq += 1
        q = fixture / 'request.tmp'
        q.write_text(json.dumps({'seq': seq, 'action': action, **kw}))
        q.replace(fixture / 'gui-request.json')

        def read():
            try:
                value = json.loads((fixture / 'gui-response.json').read_text())
                return value['result'] if value['seq'] == seq else None
            except (OSError, ValueError):
                return None
        return eventually(read, 30)

    def allocated(large=False, expected_window=None):
        if expected_window is not None:
            geometry=command('chrome')
            if [geometry['window_width'],geometry['window_height']] != list(expected_window):
                return None
        value = command('inspect')
        if not value.get('grid_size') or not value.get('area_origin'):
            return None
        width, height = value['area_size']
        cw, ch = value['cell_metrics']
        cols, rows = value['grid_size']
        if (cols, rows) != (math.floor(width/cw), math.floor(height/ch)):
            return None
        if large and not (cols > 200 and rows > 100):
            return None
        try:
            kernel = json.loads((root / 'kernel.json').read_text())
        except (OSError, ValueError):
            return None
        return value if (kernel['cols'], kernel['rows']) == (cols, rows) else None

    try:
        daemon = start([str(a.binary.resolve()), '--socket', str(sock), 'serve'], 'daemon')
        eventually(lambda: sock.exists())
        row = rpc('session.create', {'name': 'Generated native large grid',
                                     'argv': [sys.executable, str(app), str(root / 'kernel.json')]})
        gui = start([str(a.gui.resolve()), '--new-instance', '--socket', str(sock), '--session', row['session'],
                     '--fixture-dir', str(fixture)], 'gui')
        eventually(lambda: command('inspect').get('grid_size'), 30)
        command('sidebar-shell', shown=False)
        command('chrome', width=3200, height=2000, size=9, line_height=1.15, dock=False, open=False)
        view = eventually(lambda: allocated(True,(3200,2000)), 30)
        assert rpc('pane.list', {'session': row['session']})[0]['pid'] == row['pid']
        checks.append('native-full-allocation-and-kernel-grid-exceed-100x200-with-same-process')
        with Client(sock) as observer:
            observer.call('session.attach', {'session': row['session'], 'rows': view['grid_size'][1], 'cols': view['grid_size'][0]})
            observer.call('pane.sendText', {'pane': row['pane'], 'text': 'p'})
            def blue_frame():
                frame = observer.call('client.render', {'max_response_bytes': 64*1024*1024})
                grid = frame['panes'][0]['grid']
                return frame if grid['cells'][0][0][3] == grid['cells'][-1][-1][3] == [20,42,70] else None
            painted = eventually(blue_frame, 30)
        eventually(lambda: (command('inspect').get('revision') or 0) >= painted['revision'], 30)
        command('snapshot')
        from PIL import Image
        image = Image.open(fixture / 'native-rust.png').convert('RGB')
        view = command('inspect')
        ox, oy = view['area_origin']
        cw, ch = view['cell_metrics']
        cols, rows = view['grid_size']
        # The right and bottom cells are beyond the former engine bounds.
        for col, r in [(0, 0), (cols-1, 0), (0, rows-1), (cols-1, rows-1), (cols//2, rows//2)]:
            x, y = int(ox+(col+.5)*cw), int(oy+(r+.5)*ch)
            pixel = image.getpixel((x, y))
            assert max(abs(a-b) for a, b in zip(pixel, (20, 42, 70))) <= 1, (col, r, pixel)
        checks.append('actual-native-rendered-pixels-cover-all-four-corners-of-large-terminal')

        for width, height in [(2400, 1500), (3000, 1900), (1800, 1100), (3200, 2000)]:
            started = time.monotonic()
            command('chrome', width=width, height=height)
            state = eventually(lambda: allocated(height >= 2000,(width,height)), 30)
            timings.append({'requested_window': [width, height], 'actual_area': state['area_size'],
                            'grid': state['grid_size'], 'allocation_and_kernel_observed_ms': round((time.monotonic()-started)*1000, 2),
                            'max_GTK_tick_gap_ms_since_start': state['max_tick_gap_ms']})
            assert rpc('pane.list', {'session': row['session']})[0]['pid'] == row['pid']
        checks.append('repeated-native-resizes-preserve-full-logical-area-and-child')
        command('window-close')
        eventually(lambda: gui.poll() is not None)
        assert daemon.poll() is None
        assert rpc('pane.list', {'session': row['session']})[0]['pid'] == row['pid']
        checks.append('native-close-preserves-large-terminal-daemon-and-child')
        (root / 'acceptance.json').write_text(json.dumps({'status': 'PASS', 'checks': checks, 'observations': timings,
             'hashes': {n: hashlib.sha256(path.read_bytes()).hexdigest() for n, path in [('ruwac', a.binary), ('desktop', a.gui)]},
             'scope': 'Actual GTK/Wayland and decoded screenshot on an owned virtual compositor, real PTY ioctls. Timing includes polling/layout; no physical-input or display-latency claim.'}, indent=2) + '\n')
        print('NATIVE_NATURAL_GRID PASS', len(checks), 'groups', flush=True)
    finally:
        for child, log in reversed(children):
            if child.poll() is None:
                child.terminate()
                try:
                    child.wait(timeout=10)
                except subprocess.TimeoutExpired:
                    child.kill()
                    child.wait()
            log.close()
        import shutil
        shutil.rmtree(runtime)


if __name__ == '__main__':
    main()
