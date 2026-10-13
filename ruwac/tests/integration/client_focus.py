#!/usr/bin/env python3
"""Independent attached-client focus, shared PTYs, and canonical defaults."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import fcntl
import pty
import struct
import subprocess
import sys
import tempfile
import termios
import threading

from spike import Client, eventually


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--evidence', type=Path, required=True)
    args = parser.parse_args()
    args.evidence.mkdir(parents=True, exist_ok=True)
    root = Path(tempfile.mkdtemp(prefix='client-focus-', dir=args.evidence))
    root.chmod(0o700)
    binary = args.binary.resolve()
    socket = root / 'control.sock'
    env = {**os.environ, 'HOME': str(root / 'home'),
           'XDG_CONFIG_HOME': str(root / 'config'),
           'XDG_DATA_HOME': str(root / 'data'),
           'XDG_CACHE_HOME': str(root / 'cache')}
    for key in ['HOME', 'XDG_CONFIG_HOME', 'XDG_DATA_HOME', 'XDG_CACHE_HOME']:
        Path(env[key]).mkdir(mode=0o700)
    receiver = Path(__file__).resolve().parent.parent / 'fixtures/receiver.py'
    checks = []

    def argv(name):
        return [sys.executable, str(receiver), str(root / name)]

    def record(name):
        checks.append({'id': name, 'status': 'PASS'})
        print(name, 'PASS', flush=True)

    with (root / 'daemon.log').open('w') as log:
        daemon = subprocess.Popen([str(binary), '--socket', str(socket), 'serve'],
                                  env=env, stdout=log, stderr=log)
        try:
            eventually(socket.exists)
            with Client(socket) as admin, Client(socket) as one, Client(socket) as two:
                session = admin.call('session.create', {'name': 'Generated focus', 'argv': argv('a.bin')})
                pane_a, window_a, sid = session['pane'], session['window'], session['session']
                pane_b = admin.call('pane.create', {'window': window_a, 'argv': argv('b.bin')})['pane']
                for pane in [pane_a, pane_b]:
                    admin.call('pane.wait', {'pane': pane, 'contains': 'READY', 'timeout_ms': 5000})
                admin.call('pane.select', {'pane': pane_a})
                for client in [one, two]:
                    client.call('session.attach', {'session': sid, 'rows': 30, 'cols': 100})
                one.call('pane.select', {'pane': pane_b})
                assert one.call('client.render', {})['active_pane'] == pane_b
                assert two.call('client.render', {})['active_pane'] == pane_a
                one.call('client.input', {'pane': one.call('client.render', {})['active_pane'], 'kind': 'keyboard', 'text': 'ONE'})
                two.call('client.input', {'pane': two.call('client.render', {})['active_pane'], 'kind': 'keyboard', 'text': 'TWO'})
                eventually(lambda: (root / 'a.bin').read_bytes() == b'TWO' and (root / 'b.bin').read_bytes() == b'ONE')
                assert next(w for w in one.call('window.list', {'session': sid}) if w['id'] == window_a)['active_pane'] == pane_b
                assert next(w for w in two.call('window.list', {'session': sid}) if w['id'] == window_a)['active_pane'] == pane_a
                assert next(w for w in admin.call('window.list', {'session': sid}) if w['id'] == window_a)['active_pane'] == pane_b
                record('FOCUS-PANE-INPUT-AND-PROJECTIONS')

                window_b = admin.call('window.create', {'session': sid, 'argv': argv('window-b.bin')})
                window_c = admin.call('window.create', {'session': sid, 'argv': argv('window-c.bin')})
                assert one.call('client.render', {})['window'] == window_a
                assert two.call('client.render', {})['window'] == window_a
                one.call('window.select', {'window': window_b['window']})
                two.call('window.select', {'window': window_c['window']})
                assert one.call('client.render', {})['window'] == window_b['window']
                assert two.call('client.render', {})['window'] == window_c['window']
                assert one.call('window.previous', {'session': sid})['selected'] == window_a
                assert two.call('client.render', {})['window'] == window_c['window']
                assert one.call('window.cycle', {'session': sid})['selected'] == window_b['window']
                assert two.call('window.cycle', {'session': sid, 'previous': True})['selected'] == window_b['window']
                record('FOCUS-WINDOW-CYCLE-PREVIOUS')

                for client in [one, two]:
                    client.call('window.select', {'window': window_a})
                one.call('pane.select', {'pane': pane_a})
                two.call('pane.select', {'pane': pane_b})
                created = one.call('pane.create', {'window': window_a, 'orientation': 'vertical', 'argv': argv('split.bin')})
                assert one.call('client.render', {})['active_pane'] == created['pane']
                assert two.call('client.render', {})['active_pane'] == pane_b
                panes = {p['id']: p for p in admin.call('pane.list', {'session': sid})}
                assert panes[pane_a]['cols'] == 50 and panes[pane_b]['cols'] == 100
                record('FOCUS-IMPLICIT-SPLIT-TARGET')

                one.call('pane.select', {'pane': pane_a})
                one.call('pane.layout', {'window': window_a, 'zoom': True})
                assert one.call('client.render', {})['zoom'] is True
                assert two.call('client.render', {})['zoom'] is False
                two.call('pane.layout', {'window': window_a, 'zoom': True})
                assert one.call('client.render', {})['active_pane'] == pane_a
                assert two.call('client.render', {})['active_pane'] == pane_b
                two.call('client.resize', {'rows': 20, 'cols': 90})
                panes = {p['id']: p for p in admin.call('pane.list', {'session': sid})}
                assert all((panes[p]['rows'], panes[p]['cols']) == (20, 90) for p in [pane_a, pane_b])
                record('FOCUS-ZOOM-AND-MINIMUM-PTY-SIZING')

                admin.call('pane.close', {'pane': pane_a})
                assert one.call('client.render', {})['active_pane'] == pane_b
                assert two.call('client.render', {})['active_pane'] == pane_b
                stale = one.call('client.input', {'pane': pane_a, 'kind': 'keyboard', 'text': 'STALE'}, error='STALE_ID')
                assert stale
                one.call('window.select', {'window': window_b['window']})
                two.call('window.select', {'window': window_c['window']})
                admin.call('pane.close', {'pane': window_b['pane']})
                assert one.call('client.render', {})['window'] == window_a
                assert two.call('client.render', {})['window'] == window_c['window']
                assert one.call('window.previous', {'session': sid})['selected'] == window_c['window']
                record('FOCUS-CLOSED-TARGET-REPAIR')

                # A typed idempotency key is scoped to a client's UI selections.
                key = 'generated-independent-selection'
                one.call('window.select', {'window': window_a}, key=key)
                two.call('window.select', {'window': window_c['window']}, key=key)
                assert one.call('client.render', {})['window'] == window_a
                assert two.call('client.render', {})['window'] == window_c['window']
                one.call('window.select', {'window': window_c['window']}, key=key, error='IDEMPOTENCY_CONFLICT')
                record('FOCUS-IDEMPOTENCY-SCOPES')

                # Shared mutations retain global deduplication across clients.
                creation = {'session': sid, 'argv': argv('idempotent-window.bin')}
                first = one.call('window.create', creation, key='generated-shared-create')
                second = two.call('window.create', creation, key='generated-shared-create')
                assert first == second
                windows = admin.call('window.list', {'session': sid})
                assert sum(w['id'] == first['window'] for w in windows) == 1
                assert one.call('client.render', {})['window'] == first['window']
                assert two.call('client.render', {})['window'] == window_c['window']
                two.call('window.create', {**creation, 'name': 'different'},
                         key='generated-shared-create', error='IDEMPOTENCY_CONFLICT')
                one.call('window.select', {'window': window_a})
                two.call('window.select', {'window': window_c['window']})
                record('FOCUS-SHARED-CREATION-IDEMPOTENCY')

                foreign = admin.call('session.create', {'name': 'Generated other session', 'argv': argv('foreign.bin')})
                one.call('client.input', {'pane': foreign['pane'], 'kind': 'keyboard', 'text': 'WRONG'}, error='TARGET_SESSION')
                assert one.call('client.render', {})['session'] == sid
                record('FOCUS-FOREIGN-SESSION-REFUSED')

                one.call('session.detach', {})
                one.call('client.render', {}, error='NOT_ATTACHED')
                one.call('session.attach', {'session': sid, 'rows': 30, 'cols': 100})
                assert one.call('client.render', {})['window'] == window_c['window']
                assert two.call('client.render', {})['window'] == window_c['window']
                one.call('window.select', {'window': window_a}, key=key)
                assert one.call('client.render', {})['window'] == window_a
                assert two.call('client.render', {})['window'] == window_c['window']
                capabilities = admin.call('capabilities', {})
                assert capabilities['independent_client_focus'] and not capabilities['shared_focus']
                record('FOCUS-REATTACH-CANONICAL-SEED')

                # Exercise actual TUI prefix dispatch and refreshed input targets.
                tui_session = admin.call('session.create', {'name': 'Generated TUI focus', 'argv': argv('tui-a.bin')})
                tui_other = admin.call('pane.create', {'window': tui_session['window'], 'argv': argv('tui-b.bin')})
                admin.call('pane.select', {'pane': tui_session['pane']})
                for pane in [tui_session['pane'], tui_other['pane']]:
                    admin.call('pane.wait', {'pane': pane, 'contains': 'READY', 'timeout_ms': 5000})
                terminals = []
                output = [bytearray(), bytearray()]

                def drain(fd, index):
                    while True:
                        try:
                            data = os.read(fd, 65536)
                        except OSError:
                            return
                        if not data:
                            return
                        output[index].extend(data)
                        del output[index][:-8192]

                def controlling_terminal():
                    os.setsid()
                    fcntl.ioctl(0, termios.TIOCSCTTY, 0)

                try:
                    for index in range(2):
                        master, slave = pty.openpty()
                        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', 30, 100, 0, 0))
                        process = subprocess.Popen([str(binary), '--socket', str(socket), 'attach', '--session', tui_session['session']],
                                                   stdin=slave, stdout=slave, stderr=slave,
                                                   env=env, preexec_fn=controlling_terminal)
                        os.close(slave)
                        terminals.append((process, master))
                        threading.Thread(target=drain, args=(master, index), daemon=True).start()
                    eventually(lambda: all(b'RUWAC_RECEIVER_READY' in value for value in output), 10)
                    os.write(terminals[0][1], b'\x01\tFIRST_TUI')
                    eventually(lambda: (root / 'tui-b.bin').read_bytes() == b'FIRST_TUI', 10)
                    os.write(terminals[1][1], b'SECOND_TUI')
                    eventually(lambda: (root / 'tui-a.bin').read_bytes() == b'SECOND_TUI', 10)
                    assert (root / 'tui-b.bin').read_bytes() == b'FIRST_TUI'
                    record('FOCUS-TWO-REAL-TUI-PREFIX-AND-INPUT')
                finally:
                    for process, fd in reversed(terminals):
                        if process.poll() is None:
                            process.terminate()
                        try:
                            process.wait(timeout=10)
                        except subprocess.TimeoutExpired:
                            process.kill()
                            process.wait()
                        os.close(fd)
            report = {'checks': checks, 'scope': 'Real daemon, independent authenticated connections and generated PTYs',
                      'binary_sha256': hashlib.sha256(binary.read_bytes()).hexdigest(),
                      'version': subprocess.check_output([str(binary), '--version'], text=True).strip()}
            (root / 'acceptance.json').write_text(json.dumps(report, indent=2) + '\n')
            print('Evidence', root)
        finally:
            daemon.terminate()
            try:
                daemon.wait(timeout=10)
            except subprocess.TimeoutExpired:
                daemon.kill()
                daemon.wait()


if __name__ == '__main__':
    main()
