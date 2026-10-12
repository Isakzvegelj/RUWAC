#!/usr/bin/env python3
"""Native SQLite migration, canonical recovery and protected read-only fallback."""
import argparse, hashlib, json, os, sqlite3, subprocess, tempfile, time
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
    start([a.gui, '--show-machines', '--socket', str(sock), '--session', session['session'], '--fixture-dir', str(root)], 'desktop')
    eventually(lambda: command('inspect'), 15)
    db = folder / 'machines.sqlite3'
    assert inspect()['rows'] == 0
    assert db.exists() and db.stat().st_mode & 0o777 == 0o600
    with sqlite3.connect(db) as c:
        assert c.execute('PRAGMA user_version').fetchone()[0] == 1
        assert c.execute('PRAGMA application_id').fetchone()[0] == 0x52555741
        assert c.execute('PRAGMA integrity_check').fetchone()[0] == 'ok'
    results.append({'id':'SQLITE-NATIVE-PRIVATE-MIGRATION','status':'PASS'})
    saved = command('profile-editor', operation='new', fields={'0':'SQLite Živjo','1':'Disposable fixture','3':'localhost'})
    assert not saved['error'], saved
    assert inspect()['rows'] == 1
    with sqlite3.connect(db) as c:
        assert c.execute('SELECT name FROM endpoints').fetchone()[0] == 'SQLite Živjo'
        document, revision, pending = c.execute('SELECT document,revision,pending FROM library').fetchone()
        assert json.loads(document)['items'] == json.loads(store.read_text())['items']
        assert not pending
    results.append({'id':'SQLITE-NATIVE-EDITOR-COMMIT','status':'PASS'})
    def restart():
        gui = children.pop()
        gui[0].terminate(); gui[0].wait(timeout=6); gui[1].close()
        start([a.gui,'--show-machines','--socket',str(sock),'--session',session['session'],'--fixture-dir',str(root)], 'desktop-restart')
        eventually(lambda: command('inspect'),15)
    # Malformed legacy data must retain the canonical inventory visibly, while
    # actual native editor callbacks refuse writes and preserve both files.
    store.write_text('{malformed')
    before = db.read_bytes()
    restart()
    assert inspect()['rows'] == 1
    changed = command('profile-editor', operation='edit', fields={'0':'Must not save'})
    assert changed['error'], changed
    assert store.read_text() == '{malformed' and db.read_bytes() == before
    results.append({'id':'SQLITE-INVALID-BRIDGE-READONLY-NATIVE','status':'PASS'})
    store.unlink()
    restart()
    assert inspect()['rows'] == 1
    assert json.loads(store.read_text())['items'][0]['name'] == 'SQLite Živjo'
    results.append({'id':'SQLITE-MISSING-BRIDGE-RECOVERY','status':'PASS'})
    # Inject the durable state of a committed SQL write whose JSON publication
    # was interrupted. The next actual application launch finishes publication.
    with sqlite3.connect(db) as c:
        document, revision, oldhash = c.execute('SELECT document,revision,mirror_hash FROM library').fetchone()
        library = json.loads(document); library['items'][0]['name'] = 'Recovered pending commit'
        canonical = json.dumps(library,ensure_ascii=False,indent=2).encode()
        c.execute('UPDATE library SET document=?,revision=?,pending=1', (canonical, revision+1))
        c.execute('UPDATE endpoints SET name=?,search_text=?,payload=?', ('Recovered pending commit','recovered pending commit localhost',json.dumps(library['items'][0]).encode()))
    restart()
    assert inspect()['rows'] == 1 and any('Recovered pending commit' in label for label in inspect()['labels'])
    assert json.loads(store.read_text())['items'][0]['name'] == 'Recovered pending commit'
    with sqlite3.connect(db) as c: assert not c.execute('SELECT pending FROM library').fetchone()[0]
    results.append({'id':'SQLITE-PENDING-PUBLICATION-NATIVE-RECOVERY','status':'PASS'})
    # Foreign database must never be repurposed. Valid legacy metadata stays
    # visible, and attempted edits are denied without changing either file.
    gui = children.pop(); gui[0].terminate(); gui[0].wait(timeout=6); gui[1].close()
    db.unlink()
    with sqlite3.connect(db) as c: c.execute('CREATE TABLE foreign_data(value TEXT)'); c.execute("INSERT INTO foreign_data VALUES('preserve')")
    db.chmod(0o600); before = db.read_bytes(); legacy = store.read_bytes()
    start([a.gui,'--show-machines','--socket',str(sock),'--session',session['session'],'--fixture-dir',str(root)],'desktop-foreign')
    eventually(lambda: command('inspect'),15)
    assert inspect()['rows'] == 1
    changed = command('profile-editor',operation='edit',fields={'0':'Must not overwrite'})
    assert changed['error'] and db.read_bytes() == before and store.read_bytes() == legacy
    results.append({'id':'SQLITE-FOREIGN-DATABASE-NATIVE-PRESERVATION','status':'PASS'})

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
assert len(results) == 6
