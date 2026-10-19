#!/usr/bin/env python3
"""Actual CLI/TUI login startup; optional owned loopback OpenSSH fixture.

All profiles, shell wrappers, SSH keys and daemons are disposable and generated.
No owner shell configuration or external host is used.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import pwd
import shlex
import socket
import subprocess
import sys
import tempfile

from spike import Client, Tui, eventually


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--daemon', type=Path)
    parser.add_argument('--evidence', type=Path, required=True)
    parser.add_argument('--run-owned-ssh-fixture', action='store_true')
    args = parser.parse_args()
    args.evidence.mkdir(parents=True, exist_ok=True)
    root = Path(tempfile.mkdtemp(prefix='cli-user-shell-', dir=args.evidence)).resolve()
    root.chmod(0o700)
    # sshd StrictModes checks every ancestor of AuthorizedKeysFile. A private
    # child of world-writable /tmp is insufficient outside the account home.
    runtime_parent = Path('/run/user') / str(os.getuid()) if args.run_owned_ssh_fixture else Path('/tmp')
    if not runtime_parent.is_dir():
        parser.error('Owned SSH fixture requires the logged-in user runtime directory')
    runtime = Path(tempfile.mkdtemp(prefix='ruwac-shell-', dir=runtime_parent))
    runtime.chmod(0o700)
    binary = args.binary.resolve()
    daemon_binary = (args.daemon or binary).resolve()
    home = root / 'home'
    home.mkdir(mode=0o700)
    (home / '.bash_profile').write_text("printf 'GENERATED_LOGIN_PROFILE_READY\\n'\nPS1='generated-login> '\n")
    selected = root / 'selected login shell'
    selected.write_text('#!/bin/sh\n[ "$1" = "-l" ] && [ "$#" = 1 ] || exit 13\n'
                        '[ -z "${NO_COLOR+x}" ] && [ "$TERM" = xterm-256color ] && [ "$COLORTERM" = truecolor ] || exit 14\n. "$HOME/.bash_profile"\nexec /bin/cat\n')
    selected.chmod(0o700)
    env = {**os.environ, 'HOME': str(home), 'SHELL': str(selected), 'TERM': 'xterm-256color',
           'RUWAC_STATE_FILE': str(root / 'state.json'), 'NO_COLOR': '1'}
    for key in ('XDG_CONFIG_HOME', 'XDG_DATA_HOME', 'XDG_CACHE_HOME'):
        path = root / key.lower()
        path.mkdir(mode=0o700)
        env[key] = str(path)
    sock = runtime / 'control.sock'
    children, views, checks = [], [], []

    def start(command, name, child_env=env):
        log = (root / (name + '.log')).open('wb')
        child = subprocess.Popen(command, env=child_env, stdout=log, stderr=log)
        children.append((child, log))
        return child

    def rpc(method, params=None):
        with Client(sock) as client:
            return client.call(method, params)

    def cli(command, extra_env=None, prefix=None, expect=True):
        result = subprocess.run((prefix or [str(binary), '--socket', str(sock)]) + command,
                                env={**env, **(extra_env or {})}, capture_output=True, timeout=25)
        assert (result.returncode == 0) == expect, (command, result.stdout, result.stderr)
        return result

    def named(name):
        return next(s for s in rpc('session.list') if s['name'] == name)

    def panes(session):
        return rpc('pane.list', {'session': session})

    def login(pane):
        return eventually(lambda: 'GENERATED_LOGIN_PROFILE_READY' in
                          rpc('pane.capture', {'pane': pane})['text'])

    def created(command, session=None, **kw):
        before = {p['id'] for p in panes(session)} if session else set()
        result = cli(command, **kw)
        if session:
            pane = next(p for p in panes(session) if p['id'] not in before)
        else:
            value = json.loads(result.stdout)['result']
            pane = {'id': value['pane']}
        login(pane['id'])
        return pane

    def case(name):
        checks.append({'id': name, 'status': 'PASS'})
        print(name, 'PASS', flush=True)

    try:
        daemon = start([str(daemon_binary), '--socket', str(sock), 'serve'], 'daemon')
        eventually(lambda: sock.exists() and rpc('health')['ok'])
        explicit = rpc('session.create', {'name': 'Explicit application colour preference',
            'argv': ['/usr/bin/env', 'NO_COLOR=1', '/bin/sh', '-c',
                     'printf "EXPLICIT_NO_COLOR=%s\\n" "$NO_COLOR"; exec /bin/cat']})
        eventually(lambda: 'EXPLICIT_NO_COLOR=1' in
                   rpc('pane.capture', {'pane': explicit['pane']})['text'])
        case('TERMINAL-EXPLICIT-APPLICATION-NO-COLOR-PRESERVED')
        cli(['-dmS', 'detached'])
        sid = named('detached')['id']
        pane = panes(sid)[0]
        login(pane['id'])
        case('CLI-DETACHED-SELECTED-LOGIN-SHELL-LITERAL-PATH')

        view = Tui(binary, sock, None, root / 'named.pty', env=env, command=['-S', 'interactive'])
        views.append(view)
        eventually(lambda: b'GENERATED_LOGIN_PROFILE_READY' in view.data)
        interactive = named('interactive')['id']
        case('CLI-S-SINGLE-COMMAND-LOGIN-PROFILE')
        view.finish()
        views.remove(view)
        before = panes(interactive)
        view = Tui(binary, sock, None, root / 'reattach.pty',
                   env={**env, 'SHELL': '/generated/nonexistent-local-shell'},
                   command=['-r', 'interactive'])
        views.append(view)
        eventually(lambda: b'GENERATED_LOGIN_PROFILE_READY' in view.data)
        assert panes(interactive)[0]['pid'] == before[0]['pid']
        view.finish()
        views.remove(view)
        case('CLI-REATTACH-KEEPS-PID-WITHOUT-SHELL-DISCOVERY')

        created(['session', 'create', '--name', 'explicit-name', '--detach', '--json'])
        created(['window', 'create', '--session', sid, '--json'], sid)
        window = rpc('window.list', {'session': sid})[-1]['id']
        created(['pane', 'split', '--window', window, '--json'], sid)
        created(['-S', 'detached', '-X', 'screen', 'next-shell'], sid)
        case('CLI-NAMED-SESSION-WINDOW-SPLIT-SCREEN-CONTROL-LOGIN')

        view = Tui(binary, sock, sid, root / 'commands.pty', env=env)
        views.append(view)
        eventually(lambda: b'GENERATED_LOGIN_PROFILE_READY' in view.data)
        for keys in (b'\x01c', b'\x01s', b'\x01|', b'\x01:screen\r', b'\x01:split -v\r'):
            before_ids = {p['id'] for p in panes(sid)}
            view.send(keys)
            fresh = eventually(lambda: next((p for p in panes(sid) if p['id'] not in before_ids), None))
            login(fresh['id'])
            eventually(lambda: b'GENERATED_LOGIN_PROFILE_READY' in view.data)
        view.finish()
        views.remove(view)
        case('TUI-NEW-WINDOW-BOTH-SPLITS-AND-COMMAND-MODE-LOGIN')

        literal = json.loads(cli(['rpc', 'session.create', '--params',
                                 json.dumps({'name': 'custom', 'argv': ['/bin/cat']})],
                                extra_env={'SHELL': '/generated/nonexistent-local-shell'}).stdout)['result']
        rpc('pane.sendText', {'pane': literal['pane'], 'text': 'EXPLICIT_ARGV_UNCHANGED\n'})
        eventually(lambda: 'EXPLICIT_ARGV_UNCHANGED' in rpc('pane.capture', {'pane': literal['pane']})['text'])
        assert 'GENERATED_LOGIN_PROFILE_READY' not in rpc('pane.capture', {'pane': literal['pane']})['text']
        case('RAW-RPC-CUSTOM-ARGV-UNCHANGED')
        count = len(rpc('session.list'))
        cli(['-dmS', 'invalid-shell'], extra_env={'SHELL': '/generated/nonexistent-local-shell'}, expect=False)
        assert len(rpc('session.list')) == count
        case('INVALID-SHELL-REFUSAL-DOES-NOT-CREATE')

        # Actual Bash, rather than only a shell wrapper, must read the generated profile.
        bash = created(['session', 'create', '--name', 'bash-login', '--detach', '--json'],
                       extra_env={'SHELL': '/bin/bash'})
        assert login(bash['id'])
        case('ACTUAL-BASH-READS-LOGIN-PROFILE')

        if args.run_owned_ssh_fixture:
            for name in ('host', 'client'):
                subprocess.run(['ssh-keygen', '-q', '-t', 'ed25519', '-N', '', '-f', str(root / name)], check=True)
                (root / name).chmod(0o600)
            authorized = runtime / 'authorized_keys'
            authorized.write_bytes((root / 'client.pub').read_bytes())
            authorized.chmod(0o600)
            with socket.socket() as probe:
                probe.bind(('127.0.0.1', 0))
                port = probe.getsockname()[1]
            config = root / 'sshd.conf'
            config.write_text(f'ListenAddress 127.0.0.1\nPort {port}\nHostKey {root}/host\n'
                              f'PidFile {runtime}/sshd.pid\nAuthorizedKeysFile {authorized}\nStrictModes yes\n'
                              'PasswordAuthentication no\nKbdInteractiveAuthentication no\nUsePAM no\n'
                              f'AllowUsers {pwd.getpwuid(os.getuid()).pw_name}\nDisableForwarding yes\nLogLevel ERROR\n')
            key = (root / 'host.pub').read_text().split()
            known = root / 'known_hosts'
            known.write_text(f'[127.0.0.1]:{port} {key[0]} {key[1]}\n')
            ssh_config = root / 'ssh.conf'
            ssh_config.write_text(f'Host generated-login-target\n HostName 127.0.0.1\n Port {port}\n'
                                  f' User {pwd.getpwuid(os.getuid()).pw_name}\n IdentityFile {root}/client\n'
                                  f' UserKnownHostsFile {known}\n GlobalKnownHostsFile /dev/null\n IdentitiesOnly yes\n')
            bin_dir = root / 'bin'
            bin_dir.mkdir(mode=0o700)
            ssh = bin_dir / 'ssh'
            ssh.write_text('#!/bin/sh\nexec /usr/bin/ssh -F ' + shlex.quote(str(ssh_config)) + ' "$@"\n')
            ssh.chmod(0o700)
            endpoint = root / 'remote-cli'
            endpoint.write_text('#!/bin/sh\nexport SHELL=' + shlex.quote(str(selected)) + '\nexec ' +
                                shlex.quote(str(binary)) + ' "$@"\n')
            endpoint.chmod(0o700)
            sshd = start(['/usr/sbin/sshd', '-D', '-e', '-f', str(config)], 'sshd')

            def listening():
                assert sshd.poll() is None, 'Owned SSH fixture failed'
                try:
                    with socket.create_connection(('127.0.0.1', port), .1):
                        return True
                except OSError:
                    return False

            eventually(listening)
            remote_prefix = [str(binary), '--host', 'generated-login-target', '--remote-binary',
                             str(endpoint), '--socket', str(sock)]
            remote_env = {'SHELL': '/generated/nonexistent-local-shell',
                          'PATH': str(bin_dir) + os.pathsep + os.environ['PATH']}
            cli(['-dmS', 'remote-login'], extra_env=remote_env, prefix=remote_prefix)
            remote_sid = named('remote-login')['id']
            login(panes(remote_sid)[0]['id'])
            created(['window', 'create', '--session', remote_sid, '--json'], remote_sid,
                    extra_env=remote_env, prefix=remote_prefix)
            remote_window = rpc('window.list', {'session': remote_sid})[-1]['id']
            created(['pane', 'split', '--window', remote_window, '--json'], remote_sid,
                    extra_env=remote_env, prefix=remote_prefix)
            case('REAL-SSH-REMOTE-SHELL-DISCOVERY-SESSION-WINDOW-SPLIT')
            assert daemon.poll() is None
        receipt = {'status': 'PASS', 'checks': checks, 'binary_sha256': hashlib.sha256(binary.read_bytes()).hexdigest(),
                   'daemon_sha256': hashlib.sha256(daemon_binary.read_bytes()).hexdigest(),
                   'scope': 'Actual private PTYs and generated profiles; optional isolated loopback SSH. No external host or physical input claim.'}
        (root / 'acceptance.json').write_text(json.dumps(receipt, indent=2) + '\n')
        print('CLI_USER_SHELL PASS', len(checks), 'groups', flush=True)
    finally:
        for view in views:
            view.finish('term')
        for child, log in reversed(children):
            if child.poll() is None:
                child.terminate()
                try:
                    child.wait(timeout=8)
                except subprocess.TimeoutExpired:
                    child.kill()
                    child.wait()
            log.close()
        # Keep generated evidence/keys private; remove only the owned transient runtime.
        import shutil
        shutil.rmtree(runtime)


if __name__ == '__main__':
    main()
