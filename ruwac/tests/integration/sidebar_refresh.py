#!/usr/bin/env python3
"""Native sidebar Save/Edit/Delete refresh with collapsed Tools and no search events."""
import argparse, hashlib, json, os, subprocess, tempfile, time
from pathlib import Path
from spike import Client, eventually

p = argparse.ArgumentParser()
p.add_argument('--binary', required=True)
p.add_argument('--gui', required=True)
p.add_argument('--evidence', type=Path, required=True)
a = p.parse_args()
runtime = Path(os.environ.get('XDG_RUNTIME_DIR', '/'))
if os.environ.get('GDK_BACKEND') != 'wayland' or not runtime.resolve().is_relative_to(a.evidence.resolve()):
    raise SystemExit('Sidebar acceptance requires an owned Wayland runtime inside the evidence directory')
root = Path(tempfile.mkdtemp(prefix='sidebar-refresh-', dir=a.evidence))
root.chmod(0o700)
env = {**os.environ, 'GTK_A11Y': 'none', 'GSETTINGS_BACKEND': 'memory'}
for key, name in [('HOME', 'home'), ('XDG_CONFIG_HOME', 'config'), ('XDG_DATA_HOME', 'data'), ('XDG_CACHE_HOME', 'cache')]:
    env[key] = str(root / name)
    Path(env[key]).mkdir(mode=0o700)
folder = Path(env['XDG_CONFIG_HOME']) / 'RUWAC'
folder.mkdir(mode=0o700)
store = folder / 'machines.json'
store.write_text(json.dumps({'schema': 1, 'items': [], 'groups': []}))
store.chmod(0o600)
sock = root / 'control.sock'
seq = 0
children = []
results = []

def start(argv, name):
    log = (root / (name + '.log')).open('wb')
    process = subprocess.Popen(argv, env=env, stdout=log, stderr=log)
    children.append((process, log))
    return process

def command(action, **kwargs):
    global seq
    seq += 1
    temp = root / 'request.tmp'
    temp.write_text(json.dumps({'seq': seq, 'action': action, **kwargs}))
    temp.replace(root / 'gui-request.json')
    def read():
        try:
            value = json.loads((root / 'gui-response.json').read_text())
            return value['result'] if value['seq'] == seq else None
        except (OSError, ValueError):
            return None
    return eventually(read, 10)

def inspect():
    value = command('profiles-control')
    assert not value['tools_expanded'], value
    return value['visible']

try:
    start([a.binary, '--socket', str(sock), 'serve'], 'daemon')
    eventually(lambda: sock.exists())
    with Client(sock) as c:
        session = c.call('session.create', {'name': 'Sidebar regression', 'argv': ['/bin/sh', '-c', 'sleep 120']})
    start([a.gui, '--socket', str(sock), '--session', session['session'], '--fixture-dir', str(root)], 'desktop')
    eventually(lambda: command('inspect'), 15)
    assert inspect()['rows'] == 0
    saved = command('profile-editor', operation='new', fields={'0': 'Saved without searching', '1': 'Disposable fixture', '3': 'localhost'})
    assert not saved['error'], saved
    rows = inspect()
    assert rows['rows'] == 1 and any('Saved without searching' in label for label in rows['labels']), rows
    assert len(json.loads(store.read_text())['items']) == 1
    results.append({'id': 'SIDEBAR-SAVE-WITHOUT-SEARCH', 'status': 'PASS'})
    edited = command('profile-editor', operation='edit', fields={'0': 'Edited without searching'})
    assert not edited['error'], edited
    rows = inspect()
    assert rows['rows'] == 1 and any('Edited without searching' in label for label in rows['labels']), rows
    results.append({'id': 'SIDEBAR-EDIT-WITHOUT-SEARCH', 'status': 'PASS'})
    deleted = command('profile-editor', operation='edit', response='delete', confirm=True)
    assert not deleted['error'], deleted
    assert inspect()['rows'] == 0
    assert json.loads(store.read_text())['items'] == []
    results.append({'id': 'SIDEBAR-DELETE-WITHOUT-SEARCH', 'status': 'PASS'})
finally:
    for process, log in reversed(children):
        if process.poll() is None:
            process.terminate()
            try:
                process.wait(timeout=6)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait(timeout=3)
        log.close()
    receipt = {'results': results, 'sha256': {name: hashlib.sha256(Path(path).read_bytes()).hexdigest() for name, path in [('ruwac', a.binary), ('ruwac-desktop', a.gui)]}, 'scope': 'Real native GTK callbacks, synthetic actions on an owned display; no owner configuration, credentials, clipboard or physical-input claim'}
    (root / 'receipt.json').write_text(json.dumps(receipt, indent=2) + '\n')
    print(json.dumps(receipt, indent=2))
assert len(results) == 3
