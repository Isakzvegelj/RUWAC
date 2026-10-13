#!/usr/bin/env python3
"""Opt-in real RDP layout restoration; no synthetic frames or clipboard access."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile
import time

p = argparse.ArgumentParser(description=__doc__)
p.add_argument('--binary', required=True)
p.add_argument('--gui', required=True)
p.add_argument('--evidence', type=Path, required=True)
p.add_argument('--library', type=Path, required=True)
p.add_argument('--profile', required=True)
p.add_argument('--authorized-external', action='store_true', required=True)
a = p.parse_args()
# Refuse accidental owner-desktop execution. Secret Service alone uses the existing bus.
runtime = Path(os.environ.get('XDG_RUNTIME_DIR', ''))
assert runtime.is_absolute() and runtime.is_dir() and runtime.stat().st_uid == os.getuid()
assert os.environ.get('GDK_BACKEND') == 'wayland'
assert os.environ.get('WAYLAND_DISPLAY') not in (None, 'wayland-0')
assert (runtime / os.environ['WAYLAND_DISPLAY']).exists()
original = a.library.read_bytes()
library = json.loads(original)
profile = next(v for v in library['items'] if v['id'] == a.profile)
assert profile['protocol'] == 'RDP' and profile.get('preferences', {}).get('trust'), 'Existing trusted RDP profile required'
root = Path(tempfile.mkdtemp(prefix='real-rdp-workspace-', dir=a.evidence))
root.chmod(0o700)
for folder in ['config/RUWAC', 'data', 'cache', 'home']:
    (root / folder).mkdir(parents=True, mode=0o700)
# Only the owned copy changes: no automatic GDM input, reconnect, or clipboard.
profile['preferences'].update(reconnect=False, gnome_auto_login=False, clipboard=0)
library['items'] = [profile]
copy = root / 'config/RUWAC/machines.json'
copy.write_text(json.dumps(library))
copy.chmod(0o600)
env = {**os.environ, 'XDG_CONFIG_HOME': str(root / 'config'),
       'XDG_DATA_HOME': str(root / 'data'), 'XDG_CACHE_HOME': str(root / 'cache'),
       'HOME': str(root / 'home'), 'GSETTINGS_BACKEND': 'memory', 'GTK_USE_PORTAL': '0'}
sock = root / 'control.sock'
children = []
seq = 0
cases = []
before = None
last = None
received_frames = []

def start(args, name):
    log = (root / (name + '.log')).open('wb')
    child = subprocess.Popen(args, env=env, stdout=log, stderr=log)
    children.append((child, log))
    return child

def cli(*args):
    result = subprocess.run([a.binary, '--socket', str(sock), *args], env=env,
                            capture_output=True, text=True, timeout=8)
    reply = json.loads(result.stdout)
    assert result.returncode == 0 and 'result' in reply, reply
    return reply['result']

def eventually(check, seconds=12):
    end = time.monotonic() + seconds
    while time.monotonic() < end:
        value = check()
        if value:
            return value
        time.sleep(.08)
    raise AssertionError('Timed out waiting for acceptance condition')

def command(action, **params):
    global seq
    seq += 1
    temporary = root / 'request.tmp'
    temporary.write_text(json.dumps({'seq': seq, 'action': action, **params}))
    temporary.replace(root / 'gui-request.json')
    def read():
        try:
            result = json.loads((root / 'gui-response.json').read_text())
            return result['result'] if result['seq'] == seq else None
        except (OSError, ValueError):
            return None
    return eventually(read)

def remote():
    return next(v for v in cli('rdp', 'list', '--json')['views'] if v['profile'] == a.profile)

def frame():
    v = remote()
    assert v['phase'] != 'error', v
    return v if v['phase'] == 'connected' and v['decoded_updates'] > 0 else None

def listing():
    return cli('workspace', 'list', '--json')

def pids():
    sessions = cli('rpc', 'session.list')
    return {s['id']: [v['pid'] for v in cli('rpc', 'pane.list', '--params', json.dumps({'session': s['id']}))] for s in sessions}

def preserved(before, baseline, identities):
    now = remote()
    assert now['phase'] == 'connected' and now['transport_id'] == before['transport_id'], now
    assert now['decoded_updates'] >= before['decoded_updates']
    assert now['verified_tls_sha256'] == before['verified_tls_sha256']
    assert pids() == baseline
    assert {v['id'] for v in listing()['views']} == identities
    return now

try:
    start([a.binary, '--socket', str(sock), 'serve'], 'daemon')
    eventually(sock.exists)
    cli('session', 'create', '--name', 'Owned RDP workspace acceptance', '--detach', '--json')
    gui = start([a.gui, '--socket', str(sock), '--new-instance', '--clipboard-disabled',
                 '--allow-rdp-control', '--allow-workspace-control', '--fixture-dir', str(root)], 'gui')
    command('inspect')
    eventually(lambda: sock.with_suffix('.gui.sock').exists())
    cli('rdp', 'open', '--profile', a.profile, '--activate', '--json')
    before = eventually(frame, 40)
    assert before['verified_tls_sha256'] and before['transport_id']
    assert before['clipboard_channel_negotiated'] is False, 'Global clipboard isolation must omit CLIPRDR'
    baseline = pids()
    identities = {v['id'] for v in listing()['views']}
    command('rdp-switch', rdp=0)
    terminal_tab = next(i for i, t in enumerate(listing()['tabs']) if any(v['tab'] == t['id'] and v['kind'] == 'terminal' for v in listing()['views']))
    assert 'error' not in command('mixed-join', tab=terminal_tab, vertical=False)
    mixed = listing()
    snapshot = cli('workspace', 'save', '--name', 'Real connected RDP mixed layout', '--json')['id']
    preserved(before, baseline, identities)
    cases.append({'id': 'REAL_RDP_JOIN_PRESERVES_TLS_TRANSPORT_AND_PTY', 'PASS': True})
    # Exercise repeated restoration of mixed layouts, including additional terminal tabs.
    assert 'error' not in command('mixed-separate')
    preserved(before, baseline, identities)
    for repeat in range(5):
        result = cli('workspace', 'restore', '--id', snapshot, '--json')
        assert result['restored'] and not result['remote_connections_started'] and not result['terminal_processes_started']
        assert {t['id'] for t in mixed['tabs']} <= {t['id'] for t in listing()['tabs']}
        command('rdp-switch', rdp=0)
        time.sleep(.2)
        preserved(before, baseline, identities)
        # A matching newly decoded display response proves post-restore wire activity.
        prior_updates = remote()['decoded_updates']
        width, height = 960 + repeat * 20, 600 + repeat * 10
        command('rdp-display', rdp=0, mode='fit', zoom=1.0, width=width, height=height)
        def fresh_frame():
            now = preserved(before, baseline, identities)
            inspected = command('rdp-inspect', rdp=0)
            return (now['decoded_updates'] > prior_updates and inspected['size'] == [width, height]
                    and 'awaiting framebuffer' not in inspected['status'])
        started = time.monotonic()
        eventually(fresh_frame, 20)
        received_frames.append({'size': [width, height], 'decoded_updates': remote()['decoded_updates'], 'elapsed_ms': round((time.monotonic() - started) * 1000, 1)})
        if repeat < 4:
            assert 'error' not in command('mixed-separate')
    cases.append({'id': 'REAL_RDP_FIVE_RESTORES_PRESERVE_CONNECTION_AND_FRAMEBUFFER', 'PASS': True})
    command('new-terminal')
    eventually(lambda: len(listing()['views']) == len(identities) + 1)
    baseline = pids()
    identities = {v['id'] for v in listing()['views']}
    cli('workspace', 'restore', '--id', snapshot, '--json')
    command('rdp-switch', rdp=0)
    preserved(before, baseline, identities)
    cases.append({'id': 'REAL_RDP_RESTORE_PRESERVES_ADDITIONAL_TERMINAL', 'PASS': True})
    capture = command('snapshot')
    assert capture['saved']
    last = remote()
    assert a.library.read_bytes() == original, 'Owner profile library changed'
finally:
    for child, log in reversed(children):
        if child.poll() is None:
            child.terminate()
            try:
                child.wait(timeout=8)
            except subprocess.TimeoutExpired:
                child.kill()
                child.wait(timeout=3)
        log.close()
    receipt = {'cases': cases, 'restored_frame_responses': received_frames,
               'transport_before': before['transport_id'] if before else None,
               'transport_after': last['transport_id'] if last else None,
               'decoded_updates_before': before['decoded_updates'] if before else None,
               'decoded_updates_after': last['decoded_updates'] if last else None,
               'native_screenshot_sha256': hashlib.sha256((root / 'native-rust.png').read_bytes()).hexdigest() if (root / 'native-rust.png').exists() else None,
               'scope': 'Real authorized RDP TLS/authentication/framebuffer and owned PTY; synthetic GTK actions; clipboard disabled; no Linux sign-in or physical-input claim',
               'hashes': {name: hashlib.sha256(Path(path).read_bytes()).hexdigest() for name, path in [('ruwac', a.binary), ('ruwac-desktop', a.gui)]},
               'owner_library_unchanged': a.library.read_bytes() == original}
    (root / 'receipt.json').write_text(json.dumps(receipt, indent=2) + '\n')
    print(json.dumps(receipt, indent=2))
