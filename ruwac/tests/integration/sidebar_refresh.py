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
selected_shell = root / 'selected-shell'
selected_shell.write_text("#!/bin/sh\nprintf '%s\\n' GENERATED_LOCAL_READY\nexec /bin/sh\n")
selected_shell.chmod(0o700)
env['SHELL'] = str(selected_shell)
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

def editor(operation='new', response='save', **kwargs):
    # A person cannot press Save until the dialog is mapped. Let GTK/Wayland
    # establish its text-input context before exercising the response callback.
    command('profile-editor', operation=operation, response='keep', **kwargs)
    eventually(lambda: any(d['fields'] and d['width'] > 0 and d['height'] > 0
                          for d in command('profile-editor', operation='layout')['dialogs']), 10)
    return command('profile-editor', operation='update', response=response, **kwargs)

def inspect():
    value = command('profiles-control')
    assert not value['tools_expanded'], value
    return value['visible']

try:
    start([a.binary, '--socket', str(sock), 'serve'], 'daemon')
    eventually(lambda: sock.exists())
    with Client(sock) as c:
        session = c.call('session.create', {'name': 'Sidebar regression', 'argv': ['/bin/sh', '-c', 'sleep 120']})
    start([a.gui, '--show-machines', '--socket', str(sock), '--session', session['session'], '--fixture-dir', str(root)], 'desktop')
    eventually(lambda: command('inspect'), 15)
    assert inspect()['rows'] == 0
    # Another process/window saves metadata with the same atomic rename as the
    # editor. Exercise the actual native reload with an untouched search field.
    def replace_library(value):
        temp = store.with_suffix('.candidate')
        temp.write_text(json.dumps(value))
        temp.chmod(0o600)
        temp.replace(store)
    replace_library({'schema': 1, 'items': [
        {'id': 'external-one', 'machine_id': 'external-one', 'name': 'Other window fedora.example.test', 'server': 'localhost', 'protocol': 'RDP', 'username': 'fixture', 'secret_reference': '', 'group': '', 'tags': [], 'favorite': False, 'last_used': 0},
        {'id': 'external-two', 'machine_id': 'external-two', 'name': 'Other window Windows', 'server': 'localhost', 'protocol': 'RDP', 'username': 'fixture', 'secret_reference': '', 'group': '', 'tags': [], 'favorite': False, 'last_used': 0}
    ], 'groups': []})
    eventually(lambda: inspect()['rows'] == 2, 10)
    results.append({'id': 'SIDEBAR-OTHER-WINDOW-WITHOUT-SEARCH', 'status': 'PASS'})
    replace_library({'schema': 999, 'items': []})
    time.sleep(1.5)
    assert inspect()['rows'] == 2
    results.append({'id': 'SIDEBAR-INVALID-RELOAD-PRESERVES-LIST', 'status': 'PASS'})
    replace_library({'schema': 1, 'items': [], 'groups': []})
    eventually(lambda: inspect()['rows'] == 0, 10)
    results.append({'id': 'SIDEBAR-RELOAD-RECOVERS-WITHOUT-SEARCH', 'status': 'PASS'})
    saved = editor(operation='new', fields={'0': 'Saved without searching', '1': 'Disposable fixture', '3': 'localhost'})
    assert not saved['error'], saved
    rows = inspect()
    assert rows['rows'] == 1 and any('Saved without searching' in label for label in rows['labels']), rows
    assert len(json.loads(store.read_text())['items']) == 1
    results.append({'id': 'SIDEBAR-SAVE-WITHOUT-SEARCH', 'status': 'PASS'})
    # Settings saved by the newer selection-sharing controls must still allow
    # creating/editing machines, retaining the opt-ins and credential identity.
    shared = json.loads(store.read_text())
    shared['items'][0]['preferences'].update(primary_sync=True, primary_paste=True,
        primary_user='fixture', primary_display='wayland-0', primary_port=0)
    retained_reference = shared['items'][0]['secret_reference']
    replace_library(shared)
    time.sleep(1.2)
    edited = editor(operation='edit', fields={'0': 'Edited without searching'})
    assert not edited['error'], edited
    rows = inspect()
    assert rows['rows'] == 1 and any('Edited without searching' in label for label in rows['labels']), rows
    results.append({'id': 'SIDEBAR-EDIT-WITHOUT-SEARCH', 'status': 'PASS'})
    edited_metadata = json.loads(store.read_text())['items'][0]
    assert edited_metadata['preferences']['primary_sync'] and edited_metadata['preferences']['primary_paste']
    assert edited_metadata['secret_reference'] == retained_reference
    results.append({'id': 'SIDEBAR-EDIT-PRESERVES-SELECTION-PREFERENCES-AND-CREDENTIAL-REFERENCE', 'status': 'PASS'})
    deleted = editor(operation='edit', response='delete', confirm=True)
    assert not deleted['error'], deleted
    assert inspect()['rows'] == 0
    assert json.loads(store.read_text())['items'] == []
    results.append({'id': 'SIDEBAR-DELETE-WITHOUT-SEARCH', 'status': 'PASS'})
    # Use the real New terminal and key handlers, then verify both the tab and
    # daemon-owned final window/session disappear after typing exit.
    def rpc(method, params=None):
        with Client(sock) as c:
            return c.call(method, params)
    before_sessions = {s['id'] for s in rpc('session.list')}
    assert command('new-terminal')['clicked']
    created = eventually(lambda: next((s for s in rpc('session.list') if s['id'] not in before_sessions), None), 10)
    pane = rpc('pane.list', {'session': created['id']})[0]
    eventually(lambda: command('inspect', view=1).get('target') == pane['id'], 10)
    eventually(lambda: 'GENERATED_LOCAL_READY' in rpc('pane.capture', {'pane': pane['id']})['text'], 10)
    for key in ['e', 'x', 'i', 't', 'Return']:
        command('key', key=key, view=1)
    eventually(lambda: all(s['id'] != created['id'] for s in rpc('session.list')), 10)
    eventually(lambda: command('inspect')['tabs'] == 1 and command('inspect').get('target') == session['pane'], 10)
    assert rpc('pane.list', {'session': session['session']})[0]['pid'] == session['pid']
    results.append({'id': 'GUI-LOCAL-EXIT-RETIRES-FINAL-SESSION-AND-PRESERVES-OTHER-PTY', 'status': 'PASS'})

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
assert len(results) == 8
