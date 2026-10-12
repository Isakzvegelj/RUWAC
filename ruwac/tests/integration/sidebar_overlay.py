#!/usr/bin/env python3
"""Native overlay/pinned sidebar geometry, XTest keyboard routing and private settings."""
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
        session = c.call('session.create', {'name': 'Sidebar regression', 'argv': ['/bin/cat']})
        second=c.call('session.create',{'name':'Second sidebar tab','argv':['/bin/cat']})
    start([a.gui, '--socket', str(sock), '--session', session['session'], '--session', second['session'], '--fixture-dir', str(root)], 'desktop')
    eventually(lambda: command('inspect'), 15)
    state=eventually(lambda: r if (r:=command('sidebar-shell'))['allocated_content_width']>500 else None)
    assert not state['shown'] and state['overlay'] and not state['pinned'],state
    baseline=state['allocated_content_width']
    assert abs(baseline-state['allocated_width'])<=2,state
    results.append({'id':'SIDEBAR-DEFAULT-HIDDEN-FULL-VIEWPORT','status':'PASS'})
    def key(chord):
        subprocess.run(['xdotool','key','--clearmodifiers',chord],env={**env,'DISPLAY':os.environ['RUWAC_XTEST_DISPLAY']},check=True)
    eventually(lambda:command('inspect')['active'])
    key('alt+1')
    key('ctrl+k');eventually(lambda:command('sidebar-shell')['shown'])
    shown=eventually(lambda:r if (r:=command('sidebar-shell'))['allocated_sidebar_width']>0 else None)
    assert shown['overlay'] and shown['allocated_content_width']==baseline,shown
    results.append({'id':'SIDEBAR-XTEST-SEARCH-OVERLAY-NO-RESIZE','status':'PASS'})
    command('sidebar-shell',width=200)
    narrow=eventually(lambda:r if 170<=(r:=command('sidebar-shell'))['allocated_sidebar_width']<=220 else None)
    command('sidebar-shell',width=400)
    wide=eventually(lambda:r if (r:=command('sidebar-shell'))['allocated_sidebar_width']>=narrow['allocated_sidebar_width']+150 else None)
    assert wide['allocated_content_width']==baseline
    results.append({'id':'SIDEBAR-NARROW-WIDE-REAL-ALLOCATION','status':'PASS','narrow':narrow['allocated_sidebar_width'],'wide':wide['allocated_sidebar_width']})
    command('sidebar-shell',pinned=True,width=300)
    pinned=eventually(lambda:r if not (r:=command('sidebar-shell'))['overlay'] and r['allocated_content_width']<baseline-150 else None)
    assert pinned['shown'],pinned
    results.append({'id':'SIDEBAR-PINNED-DOCKED-VIEWPORT','status':'PASS'})
    command('sidebar-shell',pinned=False)
    key('Escape');eventually(lambda:not command('sidebar-shell')['shown'])
    key('ctrl+k');eventually(lambda:command('sidebar-shell')['shown'])
    key('Escape');eventually(lambda:not command('sidebar-shell')['shown'])
    time.sleep(.4)
    subprocess.run(['xdotool','type','--clearmodifiers','OVERLAY-READY'],env={**env,'DISPLAY':os.environ['RUWAC_XTEST_DISPLAY']},check=True)
    def echoed():
        with Client(sock) as c:return 'OVERLAY-READY' in c.call('pane.capture',{'pane':session['pane']})['text']
    try:eventually(echoed)
    except AssertionError:
        with Client(sock)as c:diagnostic={"inspect":command("inspect"),"first":c.call("pane.capture",{"pane":session["pane"]}),"second":c.call("pane.capture",{"pane":second["pane"]})}
        (root/"keyboard-diagnostic.json").write_text(json.dumps(diagnostic,indent=2))
        raise
    results.append({'id':'SIDEBAR-ESCAPE-RESTORES-REAL-PTY-KEYBOARD','status':'PASS'})
    key('alt+1')
    command('sidebar-shell',shown=True)
    key('alt+1');key('alt+2');eventually(lambda:not command('sidebar-shell')['shown'])
    results.append({'id':'SIDEBAR-TAB-SELECTION-AUTO-HIDE','status':'PASS'})
    command('sidebar-shell',pinned=True,width=220)
    ui=folder/'ui.json';data=json.loads(ui.read_text());assert data['width']==220 and data['pinned'] and ui.stat().st_mode&0o777==0o600,data
    gui=children.pop();command('window-close');eventually(lambda:gui[0].poll() is not None);gui[1].close()
    with Client(sock) as c:assert c.call('health')['panes']>=1
    start([a.gui,'--socket',str(sock),'--session',session['session'],'--session',second['session'],'--fixture-dir',str(root)],'desktop-restart')
    eventually(lambda:command('inspect'),15)
    restored=command('sidebar-shell');assert restored['pinned'] and restored['width']==220 and restored['shown'] and not restored['overlay'],restored
    results.append({'id':'SIDEBAR-RESTART-PREFERENCES-GUI-CLOSE-KEEPS-DAEMON','status':'PASS'})
    gui=children.pop();command('window-close');eventually(lambda:gui[0].poll() is not None);gui[1].close()
    ui.write_text('{"schema":99,"width":220,"pinned":false}');before=ui.read_bytes()
    start([a.gui,'--socket',str(sock),'--session',session['session'],'--fixture-dir',str(root)],'desktop-foreign-settings')
    eventually(lambda:command('inspect'),15);command('sidebar-shell',width=210,pinned=True)
    assert ui.read_bytes()==before
    results.append({'id':'SIDEBAR-UNKNOWN-SETTINGS-PRESERVED','status':'PASS'})

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
