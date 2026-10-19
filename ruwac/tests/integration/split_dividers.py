#!/usr/bin/env python3
"""Actual PTY split dividers and blue status, including nested RUWAC rendering."""
import argparse
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
from spike import Client, Tui, eventually

p = argparse.ArgumentParser()
p.add_argument('--binary', type=Path, required=True)
p.add_argument('--evidence', type=Path, required=True)
a = p.parse_args()
root = Path(tempfile.mkdtemp(prefix='split-dividers-', dir=a.evidence))
root.chmod(0o700)
sock = root / 'control.sock'
binary = a.binary.resolve()
env = {**os.environ, 'TERM': 'xterm-256color', 'RUWAC_STATE_FILE': str(root / 'state.json')}
for key in ['HOME','XDG_CONFIG_HOME','XDG_CACHE_HOME','XDG_DATA_HOME']:
    folder=root/key.lower();folder.mkdir(mode=0o700);env[key]=str(folder)
for key in ['RUWAC_SHORTCUTS','RUWAC_KEYMAP','ENV','BASH_ENV','TMUX','STY']:
    env.pop(key,None)
receiver = root / 'receiver.py'
receiver.write_text("""import os,signal,tty
tty.setraw(0)
def paint(*_):
 cols=os.get_terminal_size(0).columns
 os.write(1, ('\\x1b[0;48;2;20;40;60m\\x1b[2J\\x1b[HL'+' '*(cols-2)+'Z').encode())
signal.signal(signal.SIGWINCH,paint)
paint()
while os.read(0,4096):pass
""")
views = []
checks = []
log = (root / 'daemon.log').open('w')
daemon = subprocess.Popen([str(binary), '--socket', str(sock), 'serve'], env=env, stdout=log, stderr=log)


def rpc(method, params=None):
    with Client(sock) as client:
        return client.call(method, params)


def render(session):
    row=next(s for s in rpc('session.list') if s['id']==session)
    with Client(sock)as client:
        client.call('session.attach',{'session':session,'rows':row['rows'],'cols':row['cols']})
        return client.call('client.render')


try:
    eventually(sock.exists)
    inner = rpc('session.create', {'name': 'generated-inner', 'rows': 23, 'cols': 80, 'argv': [sys.executable, str(receiver)]})
    pane = rpc('pane.create', {'window': inner['window'], 'orientation': 'vertical', 'argv': [sys.executable, str(receiver)]})
    rpc('pane.create', {'window': inner['window'], 'pane': pane['pane'], 'orientation': 'horizontal', 'argv': [sys.executable, str(receiver)]})
    view = Tui(binary, sock, inner['session'], root / 'direct.pty', env=env)
    views.append(view)
    eventually(lambda: '│'.encode() in view.data and '─'.encode() in view.data)
    eventually(lambda: b'\x1b[0;38;2;16;32;48;48;2;152;198;232m' in view.data)
    for item in rpc('pane.list', {'session': inner['session']}):
        def corners():
            cells = next(p for p in render(inner['session'])['panes'] if p['id']==item['id'])['grid']['cells'][0]
            return cells[0][0] == 'L' and cells[-1][0] == 'Z'
        eventually(corners)
    checks.append('real-PTY-horizontal-vertical-dividers-blue-status-and-unobscured-application-corners')
    assert view.finish() == 0
    views.remove(view)
    outer = rpc('session.create', {'name': 'generated-outer', 'rows': 24, 'cols': 80, 'argv': [str(binary), '--socket', str(sock), 'attach', '--session', inner['session']]})
    view = Tui(binary, sock, outer['session'], root / 'nested.pty', env=env)
    views.append(view)
    def nested():
        grid = render(outer['session'])['panes'][0]['grid']
        text = ''.join(cell[0] for row in grid['cells'] for cell in row)
        return grid if '│' in text and '─' in text and 'generated-inner' in text else None
    grid = eventually(nested)
    assert any(cell[3] == [152, 198, 232] for row in grid['cells'] for cell in row), 'nested accent-blue background missing'
    checks.append('nested-RUWAC-canonical-grid-retains-divider-glyphs-and-light-blue-status-colour')
    (root / 'acceptance.json').write_text(json.dumps({'status': 'PASS', 'checks': checks, 'scope': 'Actual generated daemon, kernel PTYs and nested CLI; no physical desktop claim'}, indent=2) + '\n')
    print('Split dividers PASS', len(checks), 'groups')
finally:
    for view in views:
        view.finish('kill')
    if sock.exists():
        rpc('daemon.stop')
    daemon.wait(timeout=10)
    log.close()
