#!/usr/bin/env python3
"""Authorized SSH-host TUI acceptance; existing keys and verified host keys only."""
import argparse, fcntl, hashlib, json, os, pty, select, shlex, struct, subprocess, time
from pathlib import Path

p = argparse.ArgumentParser()
p.add_argument('--host', required=True)
p.add_argument('--binary', required=True)
p.add_argument('--receiver', required=True)
p.add_argument('--remote-evidence', required=True)
p.add_argument('--evidence', type=Path, required=True)
a = p.parse_args()
if a.host.startswith('-'):
    p.error('host cannot be an option')
ssh = ['ssh', '-o', 'BatchMode=yes', '-o', 'StrictHostKeyChecking=yes',
       '-o', 'ConnectTimeout=10', a.host]

def remote(args):
    return subprocess.check_output(ssh + [shlex.join(args)], timeout=15)

root = remote(['python3', '-c',
               'import tempfile,sys; print(tempfile.mkdtemp(prefix="ssh-tui-",dir=sys.argv[1]))',
               a.remote_evidence]).decode().strip()
sock = root + '/control.sock'

def rpc(method, params=None):
    reply = json.loads(remote([a.binary, '--socket', sock, 'rpc', method,
                               '--params', json.dumps(params or {}, ensure_ascii=False)]))
    assert 'error' not in reply, reply
    return reply['result']

def wait(fn, timeout=10):
    end = time.monotonic() + timeout
    while time.monotonic() < end:
        value = fn()
        if value:
            return value
        time.sleep(.03)
    raise AssertionError('SSH condition timed out')

clients = []
transcripts = []

def attach(session):
    master, slave = pty.openpty()
    fcntl.ioctl(slave, 0x5414, struct.pack('HHHH', 31, 105, 0, 0))
    # A real remote SSH PTY, not just a process spawned on the host.
    args = ssh[:-1] + ['-tt', ssh[-1], shlex.join(
        [a.binary, '--socket', sock, 'attach', '--session', session])]
    child = subprocess.Popen(args, stdin=slave, stdout=slave, stderr=slave)
    os.close(slave)
    item = (child, master, bytearray())
    clients.append(item)
    wait(lambda: drain(item) and b'\x1b[H\x1b[2J' in item[2])
    return item

def drain(item):
    while select.select([item[1]], [], [], .03)[0]:
        try:
            data = os.read(item[1], 65536)
        except OSError:
            break
        if not data:
            break
        item[2].extend(data)
    return True

def detach(item):
    os.write(item[1], b'\x01d')
    item[0].wait(timeout=10)
    drain(item)
    assert item[0].returncode == 0, bytes(item[2][-500:])
    transcripts.append(bytes(item[2]))
    os.close(item[1])
    clients.remove(item)

receipt = {'host': a.host, 'transport': 'OpenSSH authenticated key; real remote PTY',
           'cases': [], 'remote_evidence': root}
started = False
try:
    remote([a.binary, '--socket', sock, 'start'])
    started = True
    session = rpc('session.create', {'name': 'SSH acceptance', 'argv':
                  ['python3', a.receiver, root + '/receiver.bin']})
    rpc('pane.wait', {'pane': session['pane'], 'contains': 'READY', 'timeout_ms': 5000})
    initial_pid = rpc('pane.list', {'session': session['session']})[0]['pid']
    expected = b''
    for index in range(3):
        client = attach(session['session'])
        text = f'SSH_{index}_Živjo_🦀'.encode()
        os.write(client[1], text)
        expected += text
        wait(lambda: remote(['python3', '-c',
                             'import pathlib,sys; sys.stdout.buffer.write(pathlib.Path(sys.argv[1]).read_bytes())',
                             root + '/receiver.bin']) == expected)
        wait(lambda: drain(client) and text.hex().encode() in client[2])
        pane = rpc('pane.list', {'session': session['session']})[0]
        assert pane['pid'] == initial_pid and pane['rows'] == 30 and pane['cols'] == 105, pane
        detach(client)
        assert rpc('pane.list', {'session': session['session']})[0]['pid'] == initial_pid
        receipt['cases'].append({'id': f'SSH-TUI-{index}', 'status': 'PASS',
                                 'same_child_pid': initial_pid, 'unicode_exact': True,
                                 'remote_pty_rows': 31, 'pane_rows': 30, 'cols': 105})
    receipt['receiver_sha256'] = hashlib.sha256(expected).hexdigest()
    receipt['binary_sha256'] = remote(['sha256sum', a.binary]).decode().split()[0]
finally:
    for child, master, data in clients:
        if child.poll() is None:
            child.terminate()
            child.wait(timeout=5)
        os.close(master)
    if started:
        rpc('daemon.stop')
    a.evidence.mkdir(parents=True, exist_ok=True)
    for index, data in enumerate(transcripts):
        (a.evidence / f'ssh-tui-{index}.pty').write_bytes(data)
    (a.evidence / 'ssh-terminal-acceptance.json').write_text(json.dumps(receipt, indent=2) + '\n')
print(json.dumps(receipt, indent=2))
