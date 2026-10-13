#!/usr/bin/env python3
"""Actual OpenSSH gateway and libssh2 target, with generated trust and identities."""
import argparse
import hashlib
import json
import os
import pwd
import signal
import socket
import subprocess
import sys
import tempfile
import time
import traceback
from pathlib import Path


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--driver', required=True)
    parser.add_argument('--evidence', type=Path, required=True)
    args = parser.parse_args()
    args.evidence.mkdir(parents=True, exist_ok=True)
    root = Path(tempfile.mkdtemp(prefix='ssh-jump-', dir=args.evidence)).resolve()
    root.chmod(0o700)
    auth = Path(tempfile.mkdtemp(prefix='ruwac-jump-auth-', dir=f'/run/user/{os.getuid()}'))
    auth.chmod(0o700)
    servers, logs, results = [], [], []
    user = pwd.getpwuid(os.getuid()).pw_name
    ports = {}
    for name in ('gateway', 'target'):
        while True:
            with socket.socket() as sock:
                sock.bind(('127.0.0.1', 0))
                port = sock.getsockname()[1]
            if port not in ports.values():
                ports[name] = port
                break
    for name in ('gateway-host', 'target-host', 'gateway-client', 'target-client', 'wrong-host'):
        subprocess.run(['ssh-keygen', '-q', '-t', 'ed25519', '-N', '', '-f', str(root / name)], check=True)
        (root / name).chmod(0o600)
    for name in ('gateway', 'target'):
        authorized = auth / name
        authorized.write_bytes((root / f'{name}-client.pub').read_bytes())
        authorized.chmod(0o600)
        config = root / f'{name}-sshd.conf'
        config.write_text(f'ListenAddress 127.0.0.1\nPort {ports[name]}\nHostKey {root}/{name}-host\n'
                          f'PidFile {root}/{name}.pid\nAuthorizedKeysFile {authorized}\nStrictModes yes\n'
                          f'PasswordAuthentication no\nKbdInteractiveAuthentication no\nUsePAM no\nPermitRootLogin no\n'
                          f'AllowUsers {user}\nAllowTcpForwarding local\nSubsystem sftp internal-sftp\nLogLevel ERROR\n')
        key = (root / f'{name}-host.pub').read_text().split()
        known = root / f'{name}-known'
        known.write_text(f'[127.0.0.1]:{ports[name]} {key[0]} {key[1]}\n')
        known.chmod(0o600)
    gateway_config = root / 'gateway-client.conf'
    gateway_config.write_text(f'Host generated-jump generated-other\n HostName 127.0.0.1\n'
                              f' Port {ports["gateway"]}\n User {user}\n IdentityFile {root}/gateway-client\n'
                              f' IdentitiesOnly yes\n UserKnownHostsFile {root}/gateway-known\n GlobalKnownHostsFile /dev/null\n')
    gateway_config.chmod(0o600)
    # Fixture-only executable shim selects the disposable client configuration.
    # Production arguments, authentication and anonymous socket transport are unchanged.
    shim = root / 'bin'
    shim.mkdir(mode=0o700)
    executable = shim / 'ssh'
    executable.write_text(f'#!{sys.executable}\nimport os,sys\n'
                          f'if "-W" in sys.argv:\n with open({str(root / "gateway-pids")!r},"a") as f:f.write(str(os.getpid())+"\\n")\n'
                          f'os.execv("/usr/bin/ssh",["ssh","-F",{str(gateway_config)!r},*sys.argv[1:]])\n')
    executable.chmod(0o700)
    env = {**os.environ, 'PATH': f'{shim}:{os.environ["PATH"]}', 'HOME': str(root / 'home'),
           'XDG_STATE_HOME': str(root / 'state')}
    Path(env['HOME']).mkdir(mode=0o700)
    profile = {'id': 'generated-endpoint', 'machine_id': 'generated-machine', 'name': 'Generated target',
               'group': 'Generated', 'protocol': 'SSH', 'server': f'127.0.0.1:{ports["target"]}',
               'username': user, 'secret_reference': 'generated-unused', 'favorite': False, 'last_used': 0, 'tags': [],
               'preferences': {'identity_file': str(root / 'target-client'), 'known_hosts': str(root / 'target-known'),
                               'ssh_jump_alias': 'generated-jump'}}
    profile_file = root / 'profile.json'
    remote = root / 'remote'
    remote.mkdir(mode=0o700)

    def save():
        profile_file.write_text(json.dumps(profile))
        profile_file.chmod(0o600)

    def invoke(action, *extra):
        save()
        return json.loads(subprocess.check_output([args.driver, action, str(profile_file), *map(str, extra)],
                                                 env=env, text=True, timeout=90))

    def good(action, *extra):
        value = invoke(action, *extra)
        assert value['ok'], value
        return value['result']

    def case(name, test):
        start = time.monotonic()
        try:
            detail = test()
            results.append({'id': name, 'status': 'PASS', 'elapsed_ms': round((time.monotonic() - start) * 1000, 1), 'detail': detail})
        except Exception:
            results.append({'id': name, 'status': 'FAIL', 'error': traceback.format_exc()})
        print(name, results[-1]['status'], flush=True)

    def digest(path):
        return hashlib.sha256(path.read_bytes()).hexdigest()

    try:
        for name in ('gateway', 'target'):
            log = (root / f'{name}.log').open('wb')
            logs.append(log)
            server = subprocess.Popen(['/usr/sbin/sshd', '-D', '-e', '-f', str(root / f'{name}-sshd.conf')],
                                      stdout=log, stderr=log, start_new_session=True)
            servers.append(server)
            deadline = time.monotonic() + 5
            while True:
                assert server.poll() is None, 'Disposable SSH server failed'
                try:
                    with socket.create_connection(('127.0.0.1', ports[name]), .1):
                        break
                except OSError:
                    assert time.monotonic() < deadline, 'Disposable SSH startup timeout'
                    time.sleep(.02)

        def sftp():
            source = root / 'source.bin'
            source.write_bytes(bytes(range(256)) * 4096)
            target, copy = remote / 'payload.bin', root / 'copy.bin'
            good('list', remote)
            good('upload', source, target, 'skip')
            good('download', copy, target, 'skip')
            assert digest(source) == digest(target) == digest(copy)
            return {'bytes': source.stat().st_size, 'distinct_gateway_and_target_keys': True, 'hash_verified': True}

        case('SSH-JUMP-SFTP-UPLOAD-DOWNLOAD', sftp)

        def trust():
            for name in ('gateway', 'target'):
                known = root / f'{name}-known'
                original = known.read_text()
                try:
                    known.write_text('')
                    assert not invoke('list', remote)['ok']
                    key = (root / 'wrong-host.pub').read_text().split()
                    known.write_text(f'[127.0.0.1]:{ports[name]} {key[0]} {key[1]}\n')
                    assert not invoke('list', remote)['ok']
                finally:
                    known.write_text(original)
            good('list', remote)
            return {'unknown_and_changed_keys_refused_at_both_stages': True, 'trusted_connection_restored': True}

        case('SSH-JUMP-INDEPENDENT-HOST-TRUST', trust)

        def authentication():
            authorized = auth / 'gateway'
            original = authorized.read_bytes()
            try:
                authorized.write_bytes((root / 'target-client.pub').read_bytes())
                assert not invoke('list', remote)['ok']
            finally:
                authorized.write_bytes(original)
            original_key = profile['preferences']['identity_file']
            try:
                profile['preferences']['identity_file'] = str(root / 'gateway-client')
                assert not invoke('list', remote)['ok']
            finally:
                profile['preferences']['identity_file'] = original_key
            good('list', remote)
            return {'gateway_rejects_leaf_key': True, 'leaf_rejects_gateway_key': True}

        case('SSH-JUMP-INDEPENDENT-AUTHENTICATION', authentication)

        def resume():
            source, target = root / 'resume.bin', remote / 'resumed.bin'
            source.write_bytes(bytes(range(256)) * (16 * 1024 * 1024 // 256))
            paused = invoke('resume-upload', source, target, 'skip', 1048576)
            assert not paused['ok'] and 'paused' in paused['error'], paused
            rows = good('pending')
            assert len(rows) == 1, rows
            job = rows[0]['id']
            profile['preferences']['ssh_jump_alias'] = 'generated-other'
            try:
                assert not invoke('resume', job)['ok']
                assert not target.exists()
            finally:
                profile['preferences']['ssh_jump_alias'] = 'generated-jump'
            good('resume', job)
            assert digest(target) == digest(source)
            assert not good('pending')
            return {'resumed_bytes': source.stat().st_size, 'changed_route_refused': True, 'hash_verified': True}

        case('SSH-JUMP-RESUME-ROUTE-GUARD', resume)

        def cancellation():
            source, target = root / 'cancel.bin', remote / 'cancel-destination'
            source.touch()
            os.truncate(source, 256 * 1024 * 1024)
            target.write_bytes(b'generated original destination')
            save()
            child = subprocess.Popen([args.driver, 'upload', str(profile_file), str(source), str(target), 'overwrite', '1000'], env=env, stdout=subprocess.PIPE, text=True)
            observed = False
            try:
                deadline = time.monotonic() + 30
                while child.poll() is None:
                    assert time.monotonic() < deadline, 'Owned cancellation driver timeout'
                    for part in remote.glob('ruwac-part*'):
                        try:
                            observed |= part.stat().st_size > 0
                        except FileNotFoundError:
                            pass
                    time.sleep(.01)
                result = json.loads(child.communicate(timeout=5)[0])
            finally:
                if child.poll() is None:
                    child.terminate()
                    child.wait(timeout=5)
            assert observed, 'Cancellation did not exercise a running transfer'
            assert not result['ok'] and 'cancel' in result['error'].lower(), result
            assert target.read_bytes() == b'generated original destination'
            assert not list(remote.glob('ruwac-part*'))
            return {'actual_transfer_cancelled': True, 'destination_preserved': True, 'owned_staging_removed': True}

        case('SSH-JUMP-CANCEL-PRESERVES-DESTINATION', cancellation)

        def rsync():
            source = root / 'tree'
            source.mkdir()
            (source / 'child').mkdir()
            (source / 'child' / 'literal name.txt').write_text('generated file content\n' * 100)
            target, copy = remote / 'tree-copy', root / 'tree-returned'
            good('rsync-upload', source, target, 'skip')
            good('rsync-download', copy, target, 'skip')
            for file in source.rglob('*'):
                if file.is_file():
                    relative = file.relative_to(source)
                    assert digest(file) == digest(target / relative) == digest(copy / relative)
            return {'recursive_upload_download': True, 'hash_verified': True}

        case('SSH-JUMP-RSYNC-RECURSIVE', rsync)

        def terminal():
            argv = good('ssh-argv')['argv']
            argv[1:1] = ['-o', 'BatchMode=yes']
            completed = subprocess.run([*argv, 'printf generated-terminal-success'], env=env,
                                       capture_output=True, text=True, timeout=30)
            assert completed.returncode == 0 and completed.stdout == 'generated-terminal-success', completed.stderr
            return {'real_remote_shell_command': True, 'production_terminal_argv': True}

        case('SSH-JUMP-TERMINAL-ARGV', terminal)

        def cleanup():
            deadline = time.monotonic() + 3
            while True:
                running = []
                for pid in (root / 'gateway-pids').read_text().splitlines():
                    try:
                        state = Path(f'/proc/{pid}/stat').read_text().rsplit(')', 1)[1].split()[0]
                        if state != 'Z':
                            running.append(pid)
                    except FileNotFoundError:
                        pass
                if not running:
                    break
                assert time.monotonic() < deadline, 'Owned gateway child leaked'
                time.sleep(.02)
            return {'gateway_children_terminated': True, 'transport': 'anonymous socket pair; no local listener'}

        case('SSH-JUMP-OWNED-PROCESS-CLEANUP', cleanup)
    finally:
        for server in reversed(servers):
            if server.poll() is None:
                os.killpg(server.pid, signal.SIGTERM)
                server.wait(timeout=5)
        for log in logs:
            log.close()
        for file in auth.iterdir():
            file.unlink()
        auth.rmdir()
        report = {'cases': results, 'fixture': 'two disposable loopback OpenSSH servers with generated keys',
                  'limitations': ['No external bastion qualification', 'Gateway authentication requires a configured key or agent'],
                  'executable_config_shim': 'Fixture-only PATH shim supplies isolated gateway SSH configuration'}
        (args.evidence / 'ssh-jump-acceptance.json').write_text(json.dumps(report, indent=2))
    return 0 if len(results) == 8 and all(row['status'] == 'PASS' for row in results) else 1


if __name__ == '__main__':
    raise SystemExit(main())
