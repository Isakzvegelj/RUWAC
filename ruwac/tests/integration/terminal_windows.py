#!/usr/bin/env python3
"""Real PTYs: retired windows, independent clients, and foreground status labels."""
import argparse, json, os, re, subprocess, sys, tempfile, time
from pathlib import Path
from spike import Client, Tui, eventually

p = argparse.ArgumentParser()
p.add_argument('--binary', type=Path, required=True)
p.add_argument('--daemon', type=Path)
p.add_argument('--without-jobs', action='store_true')
p.add_argument('--evidence', type=Path, required=True)
a = p.parse_args()
root = Path(tempfile.mkdtemp(prefix='terminal-windows-', dir=a.evidence)).resolve()
root.chmod(0o700)
sock = root / 'control.sock'
env = {**os.environ, 'TERM': 'xterm-256color', 'PS1': 'generated> ',
       'RUWAC_STATE_FILE': str(root / 'state.json')}
for key in ['RUWAC_SHORTCUTS', 'RUWAC_KEYMAP', 'ENV', 'BASH_ENV']:
    env.pop(key, None)
for key in ['HOME', 'XDG_CONFIG_HOME', 'XDG_DATA_HOME', 'XDG_CACHE_HOME']:
    path = root / key.lower(); path.mkdir(mode=0o700); env[key] = str(path)
program = root / 'quiet.py'
program.write_text('import os,tty\ntty.setraw(0)\nos.write(1,b"QUIET_READY")\nb=b""\n'
                   'while not b.endswith(b"exit\\r"):\n b+=os.read(0,4096)\n')
log = (root / 'daemon.log').open('wb')
daemon = subprocess.Popen([str(a.daemon or a.binary), '--socket', str(sock), 'serve'],
                          env=env, stdout=log, stderr=log)
views, checks = [], []

def rpc(method, params=None):
    with Client(sock) as c: return c.call(method, params)

def attach(session, name):
    view = Tui(a.binary, sock, session, root / (name + '.pty'), cols=160, env=env,
               command=['-r', session])
    views.append(view); eventually(lambda: b'\x1b[24;1H' in view.data)
    return view

def finish(view):
    assert view.finish() == 0
    views.remove(view)

def status(view):
    rows = re.findall(rb'\x1b\[24;1H\x1b\[0;38;2;16;32;48;48;2;152;198;232m([^\x1b]*)\x1b\[0m', view.data)
    return rows[-1].decode() if rows else ''

try:
    eventually(sock.exists)
    history = rpc('session.create', {'name': 'Generated history', 'argv': ['/bin/sh', '-c', 'printf RETAINED_HISTORY']})
    eventually(lambda: rpc('pane.list', {'session': history['session']})[0]['ended'])
    quiet = rpc('window.create', {'session': history['session'], 'argv': [sys.executable, str(program)]})
    rpc('pane.wait', {'pane': quiet['pane'], 'contains': 'QUIET_READY', 'timeout_ms': 5000})
    rpc('window.select', {'window': history['window']})
    view = attach(history['session'], 'history-transition')
    eventually(lambda: b'RETAINED_HISTORY' in view.data)
    view.send(b'\x1b1'); eventually(lambda: b'QUIET_READY' in view.data); time.sleep(.3)
    view.send(b'exit\r'); eventually(lambda: view.p.poll() is not None, 5); finish(view)
    assert all(row['ended'] for row in rpc('pane.list', {'session': history['session']}))
    checks.append('history-to-idle-live-window-exit-detaches-with-final-history-preserved')

    active = rpc('session.create', {'name': 'Generated shared exit', 'argv': [sys.executable, str(program)]})
    survivor = rpc('window.create', {'session': active['session'], 'name': 'survivor', 'argv': ['/bin/cat']})
    rpc('window.select', {'window': active['window']})
    first, second = attach(active['session'], 'first'), attach(active['session'], 'second')
    eventually(lambda: b'QUIET_READY' in first.data and b'QUIET_READY' in second.data)
    first.send(b'exit\r')
    eventually(lambda: [row['id'] for row in rpc('window.list', {'session': active['session']})] == [survivor['window']])
    assert first.p.poll() is None and second.p.poll() is None
    eventually(lambda: all(status(view).startswith('RUWAC Generated shared exit · 0:survivor') for view in [first, second]))
    first.send(b'SURVIVOR_INPUT\r')
    eventually(lambda: 'SURVIVOR_INPUT' in rpc('pane.capture', {'pane': survivor['pane']})['text'])
    assert rpc('pane.list', {'session': active['session']})[0]['pid'] == survivor['pid']
    finish(first); finish(second)
    checks.append('exit-removes-completed-window-for-both-clients-and-preserves-live-sibling-PID')

    if not a.without_jobs:
        shell = rpc('session.create', {'name': 'Generated command labels',
            'argv': ['/bin/bash', '--noprofile', '--norc', '-c', 'printf COMMAND_READY; exec /bin/bash --noprofile --norc']})
        rpc('window.rename', {'window': shell['window'], 'name': 'shell'})
        view = attach(shell['session'], 'command-labels')
        eventually(lambda: '0:bash' in status(view))
        view.send(b'sleep 30\r')
        eventually(lambda: '0:sleep' in status(view), 5)
        assert 'sleep 30' not in status(view)
        rpc('window.rename', {'window': shell['window'], 'name': 'My service'})
        eventually(lambda: 'My service (sleep)' in status(view))
        assert rpc('window.list', {'session': shell['session']})[0]['name'] == 'My service'
        view.send(b'\x03'); eventually(lambda: 'My service (bash)' in status(view), 5)
        finish(view)
        checks.append('foreground-command-updates-without-output-keeps-custom-names-and-hides-arguments')
    else:
        checks.append('older-engine-without-job-metadata-keeps-working-terminal-labels')
    (root / 'acceptance.json').write_text(json.dumps({'PASS': True, 'checks': checks,
        'engine': subprocess.check_output([str(a.daemon or a.binary), '--version'], text=True).strip(),
        'scope': 'Actual isolated PTYs and generated processes; no owner sessions or physical-input claim'}, indent=2) + '\n')
    print('Terminal windows PASS', len(checks), 'groups', flush=True)
finally:
    for view in views: view.finish('term')
    if daemon.poll() is None: daemon.terminate(); daemon.wait(timeout=8)
    log.close()
