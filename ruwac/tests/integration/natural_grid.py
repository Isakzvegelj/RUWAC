#!/usr/bin/env python3
"""Real PTY sizes, large coloured render framing, shared views and recovery."""
import argparse
import fcntl
import hashlib
import json
import os
from pathlib import Path
import signal
import socket
import struct
import subprocess
import sys
import tempfile
import time
import uuid

from spike import Client, Tui, eventually, exact


def main():
    p = argparse.ArgumentParser()
    p.add_argument('--binary', type=Path, required=True)
    p.add_argument('--legacy', type=Path)
    p.add_argument('--previous-reader', type=Path)
    p.add_argument('--evidence', type=Path, required=True)
    a = p.parse_args()
    a.evidence.mkdir(parents=True, exist_ok=True)
    root = Path(tempfile.mkdtemp(prefix='natural-grid-', dir=a.evidence)).resolve()
    root.chmod(0o700)
    runtime = Path(tempfile.mkdtemp(prefix='ruwac-grid-', dir='/tmp'))
    runtime.chmod(0o700)
    binary = str(a.binary.resolve())
    sock = runtime / 'control.sock'
    state = root / 'state.json'
    env = {**os.environ, 'TERM': 'xterm-256color', 'RUWAC_STATE_FILE': str(state)}
    for k in ('HOME', 'XDG_CONFIG_HOME', 'XDG_CACHE_HOME', 'XDG_DATA_HOME'):
        value = root / k.lower()
        value.mkdir(mode=0o700)
        env[k] = str(value)
    children, views, results = [], [], []
    app = root / 'receiver.py'
    app.write_text('''import fcntl,json,os,signal,struct,sys,tty
from pathlib import Path
r=Path(sys.argv[1]);tty.setraw(0);generation=0
def dimensions():return struct.unpack('HHHH',fcntl.ioctl(0,0x5413,b'\\0'*8))[:2]
def report(*_):
 rows,cols=dimensions();q=r/'size.tmp';q.write_text(json.dumps({'rows':rows,'cols':cols,'pid':os.getpid()}));q.replace(r/'size.json')
signal.signal(signal.SIGWINCH,report);report();os.write(1,b'GENERATED_GRID_READY')
while True:
 data=os.read(0,4096)
 if not data:break
 with(r/'input.bin').open('ab')as f:f.write(data)
 for byte in data:
  if byte==ord('s'):report()
  if byte==ord('p'):
   generation+=1;rows,cols=dimensions();chunks=[b'\\x1b[2J']
   for y in range(rows):
    chunks.append(f'\\x1b[{y+1};1H'.encode())
    for x in range(cols):
     n=y*cols+x;red=(n+generation)%256;green=(n//256)%256;blue=(n//65536)%256
     char='Q'if n==0 else'W'if n==rows*cols-1 else'x'
     chunks.append(f'\\x1b[38;2;{red};{green};{blue}m{char}'.encode())
   chunks.append(b'\\x1b[0m');payload=b''.join(chunks)
   while payload:payload=payload[os.write(1,payload):]
   q=r/'paint.tmp';q.write_text(json.dumps({'generation':generation,'rows':rows,'cols':cols}));q.replace(r/'paint.json')
''')

    def start(executable, socket_path, name, extra_env=None):
        log = (root / (name + '.log')).open('wb')
        child = subprocess.Popen([str(executable), '--socket', str(socket_path), 'serve'],
                                 env={**env, **(extra_env or {})}, stdout=log, stderr=log)
        children.append((child, log))

        def ready():
            assert child.poll() is None, 'Owned daemon stopped during startup'
            return socket_path.exists()

        eventually(ready, 15)
        return child

    def rpc(method, params=None, error=None):
        with Client(sock) as c:
            return c.call(method, params, error=error)

    def size(rows, cols):
        def matches():
            try:
                d = json.loads((root / 'size.json').read_text())
                return d if (d['rows'], d['cols']) == (rows, cols) else None
            except (OSError, ValueError):
                return None
        return eventually(matches, 15)

    def render(c, bound=None, expect=None):
        params = {} if bound is None else {'max_response_bytes': bound}
        q = {'version': 0, 'id': uuid.uuid4().hex, 'method': 'client.render', 'params': params}
        b = json.dumps(q).encode()
        c.s.sendall(struct.pack('!I', len(b)) + b)
        n = struct.unpack('!I', exact(c.s, 4))[0]
        assert 0 < n <= (bound if isinstance(bound, int) and bound > 0 else 1024 * 1024)
        v = json.loads(exact(c.s, n))
        assert v['id'] == q['id'] and v['version'] == 0
        if expect:
            assert v['error']['code'] == expect, v
            return None, n
        assert 'error' not in v, v
        return v['result'], n

    def paint(rows, cols):
        try:
            previous=json.loads((root/'paint.json').read_text())['generation']
        except (OSError,ValueError):
            previous=0
        generation=previous+1
        rpc('pane.sendText',{'pane':row['pane'],'text':'p'})
        def delivered():
            try:
                data=json.loads((root/'paint.json').read_text())
                return data if data['generation']==generation and (data['rows'],data['cols'])==(rows,cols) else None
            except (OSError,ValueError):return None
        eventually(delivered,30)
        last=rows*cols-1
        expected=[(last+generation)%256,(last//256)%256,(last//65536)%256]
        with Client(sock)as observer:
            observer.call('session.attach',{'session':row['session'],'rows':rows,'cols':cols})
            def complete():
                frame,_=render(observer,64*1024*1024)
                grid=frame['panes'][0]['grid']
                return grid['rows']==rows and grid['cols']==cols and grid['cells'][-1][-1][0]=='W' and grid['cells'][-1][-1][2]==expected
            eventually(complete,30)

    def case(name, **evidence):
        results.append({'id': name, 'status': 'PASS', **evidence})
        print(name, 'PASS', flush=True)

    try:
        daemon = start(binary, sock, 'daemon')
        with Client(sock) as c:
            caps = c.call('capabilities')
            assert caps['terminal_size_limits'] == {'rows': 512, 'cols': 1024, 'cells': 262144}
            maximum = root / 'maximum'
            maximum.mkdir(mode=0o700)
            full = c.call('session.create', {'name':'Generated cell-budget boundary','rows':512,'cols':512,
                                             'argv':[sys.executable,str(app),str(maximum)]})
            def maximum_size():
                try:
                    value=json.loads((maximum/'size.json').read_text())
                    return value if (value['rows'],value['cols'])==(512,512) else None
                except (OSError,ValueError):return None
            assert eventually(maximum_size,15)['pid']==full['pid']
            c.call('pane.close',{'pane':full['pane']})
            case('GRID-FULL-CELL-BUDGET-ACCEPTED-BY-REAL-KERNEL-PTY')
            row = c.call('session.create', {'name': 'Generated large grid', 'rows': 160, 'cols': 320,
                                          'argv': [sys.executable, str(app), str(root)]})
            c.call('session.attach', {'session': row['session'], 'rows': 160, 'cols': 320})
            assert size(160, 320)['pid'] == row['pid']
            before = len(c.call('session.list'))
            for shape in ({'rows': 513, 'cols': 80}, {'rows': 40, 'cols': 1025},
                          {'rows': 512, 'cols': 1024}, {'rows': 0, 'cols': 80}):
                c.call('session.create', {'name': 'Refused grid', **shape}, error='LIMIT')
                c.call('client.resize', shape, error='LIMIT')
            assert len(c.call('session.list')) == before and size(160, 320)['pid'] == row['pid']
            case('GRID-REAL-PTY-DIMENSIONS-AND-NONMUTATING-BOUNDS')
            paint(160,320)
            _, refused_bytes = render(c, expect='LIMIT')
            assert c.call('health')['ok'], 'A bounded refusal must leave framing usable'
            started = time.monotonic()
            frame, frame_bytes = render(c, 64 * 1024 * 1024)
            elapsed = time.monotonic()-started
            grid = frame['panes'][0]['grid']
            assert frame_bytes > 1024 * 1024
            assert (grid['rows'], grid['cols']) == (160, 320)
            assert grid['cells'][0][0][0] == 'Q' and grid['cells'][-1][-1][0] == 'W'
            assert grid['cells'][0][0][2] == [1, 0, 0]
            for invalid in (0, 1024 * 1024 - 1, 64 * 1024 * 1024 + 1, 'large'):
                c.call('client.render', {'max_response_bytes': invalid}, error='INVALID_PARAMS')
            assert c.call('health')['ok']
            case('GRID-LARGE-TRUECOLOR-FRAME-EXPLICIT-OPT-IN-AND-CORRELATED-REFUSALS',
                 frame_bytes=frame_bytes, default_refusal_bytes=refused_bytes,
                 render_roundtrip_ms=round(elapsed*1000, 2))
            c.call('session.detach')

        view = Tui(a.binary.resolve(), sock, row['session'], root / 'large.pty', rows=161, cols=320, env=env)
        views.append(view)
        eventually(lambda: b'\x1b[160;1H' in view.data and b'\x1b[161;1H' in view.data, 25)
        assert size(160, 320)['pid'] == row['pid']
        fcntl.ioctl(view.slave, 0x5414, struct.pack('HHHH', 141, 480, 0, 0))
        assert size(140, 480)['pid'] == row['pid']
        paint(140,480)
        eventually(lambda: b'\x1b[140;1H' in view.data and b'\x1b[141;1H' in view.data, 25)
        prior = (root / 'input.bin').read_bytes()
        view.send(b'INPUT_EXACT')
        eventually(lambda: (root / 'input.bin').read_bytes() == prior + b'INPUT_EXACT')
        case('GRID-ACTUAL-TUI-FULL-HEIGHT-WIDTH-RESIZE-AND-EXACT-INPUT')

        with Client(sock) as c:
            c.call('session.attach', {'session': row['session'], 'rows': 140, 'cols': 480})
            split = c.call('pane.create', {'window': row['window'], 'orientation': 'vertical', 'argv': ['/bin/cat']})
            eventually(lambda: size(140, 239)['pid'] == row['pid'])
            panes = c.call('pane.list', {'session': row['session']})
            assert len(panes) == 2 and sorted(p['cols'] for p in panes) == [239, 240] and all(p['rows'] == 140 for p in panes)
            c.call('pane.close', {'pane': split['pane']})
            assert size(140, 480)['pid'] == row['pid']
            c.call('session.detach')
        with Client(sock) as smaller:
            smaller.call('session.attach', {'session': row['session'], 'rows': 23, 'cols': 80})
            assert size(23, 80)['pid'] == row['pid']
        assert size(140, 480)['pid'] == row['pid']
        case('GRID-LARGE-SPLITS-SMALLER-ATTACHMENT-AND-RESTORED-AREA-SAME-PID')
        view.finish()
        views.remove(view)
        paint(140,480)
        rpc('daemon.stop')
        eventually(lambda: daemon.poll() is not None, 12)
        assert json.loads(state.read_text())['schema'] == 6
        original = state.read_bytes()
        daemon = start(binary, sock, 'restored')
        with Client(sock) as c:
            c.call('session.attach', {'session': row['session'], 'rows': 140, 'cols': 480})
            restored, _ = render(c, 64 * 1024 * 1024)
            assert restored['panes'][0]['restored']
            assert restored['panes'][0]['grid']['cells'][-1][-1][0] == 'W'
            c.call('pane.sendText', {'pane': row['pane'], 'text': 'NO_INPUT'}, error='PANE_EXITED')
            c.call('client.resize', {'rows':23,'cols':80})
            c.call('daemon.checkpoint')
            saved=json.loads(state.read_text())
            assert saved['schema']==6 and saved['sessions'][0]['rows']==23
            assert saved['panes'][0]['terminal']['rows']>100
            original=state.read_bytes()
        case('GRID-LARGE-COLOURED-CHECKPOINT-RESTORES-ENDED-HISTORY')
        if a.previous_reader:
            copy = root / 'older-reader.json'
            copy.write_bytes(original)
            copy.chmod(0o600)
            log = (root / 'older-reader.log').open('wb')
            old = subprocess.Popen([str(a.previous_reader.resolve()), '--socket', str(runtime / 'old-read.sock'), 'serve'],
                                   env={**env, 'RUWAC_STATE_FILE': str(copy)}, stdout=log, stderr=log)
            children.append((old, log))
            eventually(lambda: old.poll() is not None, 12)
            assert old.returncode != 0 and copy.read_bytes() == original
            case('GRID-PREVIOUS-CHECKPOINT-READER-REFUSES-NEW-SCHEMA-WITHOUT-REWRITE')
        if a.legacy:
            legacy_sock = runtime / 'legacy.sock'
            start(a.legacy.resolve(), legacy_sock, 'legacy', {'RUWAC_STATE_FILE': str(root / 'legacy.json')})
            with Client(legacy_sock) as c:
                legacy_row = c.call('session.create', {'name': 'Preserved older engine', 'argv': ['/bin/cat']})
            legacy_view = Tui(a.binary.resolve(), legacy_sock, legacy_row['session'], root / 'legacy.pty',
                              rows=161, cols=320, env=env)
            views.append(legacy_view)
            with Client(legacy_sock) as c:
                eventually(lambda: c.call('pane.list', {'session': legacy_row['session']})[0]['rows'] == 100)
                old_pane = c.call('pane.list', {'session': legacy_row['session']})[0]
                assert old_pane['cols'] == 200 and old_pane['pid'] == legacy_row['pid']
            eventually(lambda: b'\x1b[161;1H' in legacy_view.data)
            legacy_view.finish()
            views.remove(legacy_view)
            case('GRID-OLDER-ENGINE-BOUND-FALLBACK-PRESERVES-PROCESS')
        (root / 'acceptance.json').write_text(json.dumps({'status': 'PASS', 'checks': results,
             'binary_sha256': hashlib.sha256(a.binary.read_bytes()).hexdigest(),
             'scope': 'Actual owned PTYs and Unix sockets; generated RGB grid, real kernel sizes and checkpoint files. No physical desktop claim.'}, indent=2) + '\n')
        print('NATURAL_GRID PASS', len(results), 'groups', flush=True)
    finally:
        for view in views:
            view.finish('term')
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
