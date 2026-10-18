#!/usr/bin/env python3
"""Real PTY shell exit, independent live-window fallback, and ended-history reattach."""
import argparse
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time

from spike import Client, Tui, eventually

p = argparse.ArgumentParser()
p.add_argument('--binary', type=Path, required=True)
p.add_argument('--evidence', type=Path, required=True)
a = p.parse_args()
a.evidence.mkdir(parents=True, exist_ok=True)
root = Path(tempfile.mkdtemp(prefix='terminal-exit-', dir=a.evidence)).resolve()
root.chmod(0o700)
sock = root / 'control.sock'
env = {**os.environ, 'TERM': 'xterm-256color', 'RUWAC_STATE_FILE': str(root / 'state.json')}
for key in ['RUWAC_SHORTCUTS', 'RUWAC_KEYMAP', 'ENV', 'BASH_ENV']:
    env.pop(key, None)
log = (root / 'daemon.log').open('wb')
daemon = subprocess.Popen([str(a.binary), '--socket', str(sock), 'serve'], env=env, stdout=log, stderr=log)
views, results = [], []


def rpc(method, params=None):
    with Client(sock) as client:
        return client.call(method, params)


def attach(session, name):
    view = Tui(a.binary, sock, session, root / (name + '.pty'), env=env)
    views.append(view)
    eventually(lambda: b'\x1b[H\x1b[2J' in view.data)
    return view


def shell(name):
    session = rpc('session.create', {'name': name, 'argv': ['/bin/sh', '-c', 'printf SHELL_READY; exec /bin/sh']})
    rpc('pane.wait', {'pane': session['pane'], 'contains': 'SHELL_READY', 'timeout_ms': 5000})
    return session


try:
    eventually(sock.exists)
    session = shell('Exit fixture')
    view = attach(session['session'], 'exit')
    view.send(b'exit\r')
    eventually(lambda: view.p.poll() is not None, 4)
    views.remove(view)
    assert view.finish() == 0
    assert rpc('pane.list', {'session': session['session']})[0]['ended']
    assert rpc('pane.capture', {'pane': session['pane']})['text']
    results.append('shell-exit-returns-client-and-restores-terminal-with-history-retained')
    history = attach(session['session'], 'history')
    time.sleep(.2)
    assert history.p.poll() is None
    views.remove(history)
    assert history.finish() == 0
    results.append('explicit-ended-history-attachment-remains-inspectable')
    other = shell('Other live window')
    view = attach(other['session'], 'other-window')
    survivor = rpc('window.create', {'session': other['session'], 'name': 'survivor', 'argv': ['/bin/cat']})
    view.send(b'exit\r')
    eventually(lambda: rpc('job.query', {'pane': other['pane']})['pane_ended'])
    time.sleep(.2)
    assert view.p.poll() is None
    view.send(b'SURVIVOR_INPUT\r')
    eventually(lambda: 'SURVIVOR_INPUT' in rpc('pane.capture', {'pane': survivor['pane']})['text'])
    assert next(p for p in rpc('pane.list', {'session': other['session']}) if p['id'] == survivor['pane'])['pid'] == survivor['pid']
    views.remove(view)
    assert view.finish() == 0
    results.append('exiting-current-window-selects-another-live-window-without-killing-it')
    final = rpc('session.create', {'name': 'Drain fixture', 'argv': [sys.executable, '-c',
        "import os,signal,time; signal.signal(signal.SIGHUP,signal.SIG_IGN); os.write(1,b'FINAL_CHILD_MARKER\\n'); child=os.fork(); (time.sleep(1),os.write(1,b'POST_EXIT_BACKGROUND\\n'),os._exit(0)) if child==0 else None; os._exit(0)"]})
    rpc('pane.wait', {'pane': final['pane'], 'contains': 'FINAL_CHILD_MARKER', 'timeout_ms': 5000})
    eventually(lambda: rpc('pane.list', {'session': final['session']})[0]['ended'], 2)
    ended = rpc('pane.capture', {'pane': final['pane']})['text']
    assert 'FINAL_CHILD_MARKER' in ended
    time.sleep(1.2)
    assert rpc('pane.capture', {'pane': final['pane']})['text'] == ended, 'Ended history changed after late background output'
    results.append('final-child-output-preserved-and-ended-history-ignores-late-inherited-writer')
    closed = rpc('session.create', {'name': 'Closed stdio fixture', 'argv': [sys.executable, '-c',
        'import os,time; os.close(0); os.close(1); os.close(2); time.sleep(.6); os._exit(7)']})
    time.sleep(.2)
    assert rpc('pane.list', {'session': closed['session']})[0]['ended'] is False
    eventually(lambda: rpc('pane.list', {'session': closed['session']})[0]['ended'], 2)
    assert rpc('job.query', {'pane': closed['pane']})['exit_code'] == 7
    results.append('closed-stdio-waits-for-real-process-exit-and-keeps-exit-status')
    reopened = rpc('session.create', {'name': 'Reopened stdio fixture', 'argv': [sys.executable, '-c',
        "import os,time; os.close(0); os.close(1); os.close(2); time.sleep(.2); fd=os.open('/dev/tty',os.O_WRONLY); os.write(fd,b'RESUMED_AFTER_CLOSE\\n'); os.close(fd); os._exit(9)"]})
    rpc('pane.wait', {'pane': reopened['pane'], 'contains': 'RESUMED_AFTER_CLOSE', 'timeout_ms': 5000})
    eventually(lambda: rpc('pane.list', {'session': reopened['session']})[0]['ended'], 2)
    assert rpc('job.query', {'pane': reopened['pane']})['exit_code'] == 9
    results.append('temporarily-closed-stdio-resumes-output-before-final-status')
    print('Terminal exit PASS', len(results), 'groups')
finally:
    for view in views:
        view.finish('kill')
    if daemon.poll() is None:
        rpc('daemon.stop')
        daemon.wait(timeout=5)
    log.close()
    (root / 'receipt.json').write_text(json.dumps({'results': results, 'complete': len(results) == 6,
        'scope': 'Owned real PTYs and generated sessions; daemon/history preserved'}, indent=2) + '\n')
