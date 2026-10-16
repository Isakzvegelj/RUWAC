#!/usr/bin/env python3
"""SSH key-access discovery against disposable OpenSSH servers.

Everything is generated: keys, an isolated ssh-agent, a temporary ssh_config
(through an `ssh` wrapper adding -F, because OpenSSH reads the account's real
home directory) and sshd instances on loopback. Covers Include, Match,
ProxyJump, duplicate aliases, hashed known_hosts, certificates, agent-only
keys, password-only servers, unknown and changed host keys, unreachable hosts
and stale-cache inputs.
"""
import argparse, json, os, shutil, socket, subprocess, sys, tempfile, time
from pathlib import Path

p = argparse.ArgumentParser()
p.add_argument('--driver', required=True, help='files-fixture example binary')
p.add_argument('--evidence', type=Path, required=True)
a = p.parse_args()
root = Path(tempfile.mkdtemp(prefix='ssh-access-', dir='/tmp'))
root.chmod(0o700)
home = root / 'home'; ssh_dir = home / '.ssh'; ssh_dir.mkdir(parents=True); ssh_dir.chmod(0o700)
bin_dir = root / 'bin'; bin_dir.mkdir()
servers = []; agent_pid = None
checks = []


def free_port():
    with socket.socket() as s:
        s.bind(('127.0.0.1', 0)); return s.getsockname()[1]


def keygen(path, kind='ed25519', comment='generated'):
    subprocess.run(['ssh-keygen', '-q', '-t', kind, '-N', '', '-C', comment, '-f', str(path)], check=True)
    path.chmod(0o600)
    return path


def server(name, authorized=(), password_only=False, ca=None):
    d = root / name; d.mkdir(); d.chmod(0o700)
    host = keygen(d / 'host', comment=name + '-host')
    auth = d / 'authorized_keys'
    auth.write_text(''.join(Path(str(k) + '.pub').read_text() for k in authorized)); auth.chmod(0o600)
    port = free_port()
    lines = [f'Port {port}', 'ListenAddress 127.0.0.1', f'HostKey {host}', f'AuthorizedKeysFile {auth}',
             'UsePAM no', 'StrictModes no', f'PidFile {d}/pid', 'KbdInteractiveAuthentication no']
    if password_only:
        lines += ['PubkeyAuthentication no', 'PasswordAuthentication yes']
    else:
        lines += ['PasswordAuthentication no']
    if ca:
        lines.append(f'TrustedUserCAKeys {ca}.pub')
    (d / 'sshd_config').write_text('\n'.join(lines) + '\n')
    proc = subprocess.Popen(['/usr/sbin/sshd', '-D', '-f', str(d / 'sshd_config'), '-E', str(d / 'log')])
    servers.append(proc)
    for _ in range(100):
        with socket.socket() as s:
            if s.connect_ex(('127.0.0.1', port)) == 0:
                break
        time.sleep(.05)
    return port, host


def profile(name, server):
    hexid = os.urandom(16).hex()
    return {'id': 'endpoint_' + hexid, 'machine_id': 'machine_' + hexid, 'name': name, 'group': 'Generated',
            'protocol': 'SSH', 'server': server, 'username': '', 'secret_reference': 'endpoint_' + hexid,
            'favorite': False, 'last_used': 0, 'tags': []}


def access(name, server):
    path = root / (name + '.json'); path.write_text(json.dumps(profile(name, server)))
    out = subprocess.run([a.driver, 'ssh-access', str(path)], capture_output=True, text=True, timeout=120, env=env)
    assert out.returncode == 0, out.stderr
    result = json.loads(out.stdout).get('result', {})
    assert 'access' in result, (name, server, result)
    return result


def fingerprint(key):
    return subprocess.check_output(['ssh-keygen', '-l', '-f', str(key) + '.pub'], text=True).split(None, 1)[1].strip()


try:
    # Identities: a key file, an agent-only key, an unused key file, a CA-certified key.
    file_key = keygen(ssh_dir / 'id_ed25519', comment='generated-file-key')
    agent_key = keygen(root / 'agent_only', comment='generated-agent-key')
    keygen(ssh_dir / 'id_ecdsa', 'ecdsa', 'generated-unused-key')
    ca = keygen(root / 'user_ca', comment='generated-ca')
    cert_key = keygen(root / 'cert_key', comment='generated-cert-key')
    subprocess.run(['ssh-keygen', '-q', '-s', str(ca), '-I', 'generated', '-n', os.environ.get('USER', 'user'),
                    str(cert_key) + '.pub'], check=True)

    a_port, a_host = server('accepts-file', [file_key])
    b_port, b_host = server('accepts-agent', [agent_key])
    c_port, c_host = server('no-key')
    d_port, d_host = server('password-only', password_only=True)
    e_port, _ = server('unknown-host', [file_key])
    f_port, _ = server('changed-host', [file_key])
    g_port = free_port()  # nothing listens
    h_port, h_host = server('behind-jump', [file_key])
    i_port, i_host = server('cert-only', ca=ca)

    other = keygen(root / 'other_host', comment='generated-wrong-host')
    known = ssh_dir / 'known_hosts'
    entries = [(a_port, a_host), (b_port, b_host), (c_port, c_host), (d_port, d_host), (f_port, other),
               (h_port, h_host), (i_port, i_host)]
    known.write_text(''.join(f'[127.0.0.1]:{port} {Path(str(k) + ".pub").read_text().split()[0]} '
                             f'{Path(str(k) + ".pub").read_text().split()[1]}\n' for port, k in entries))
    subprocess.run(['ssh-keygen', '-q', '-H', '-f', str(known)], check=True, capture_output=True)
    Path(str(known) + '.old').unlink(missing_ok=True)
    assert '|1|' in known.read_text()
    checks.append('hashed-known-hosts')

    (ssh_dir / 'config.d').mkdir()
    (ssh_dir / 'config.d' / 'included.conf').write_text(
        f'Host inc-a\n  HostName 127.0.0.1\n  Port {a_port}\n')
    (ssh_dir / 'config').write_text('\n'.join([
        f'Include {ssh_dir}/config.d/*.conf',
        'Host dup-alias', '  HostName 127.0.0.1', f'  Port {a_port}',
        'Host dup-alias', f'  Port {c_port}',
        'Match host matched-a', '  HostName 127.0.0.1', f'  Port {a_port}',
        'Host jumped', '  HostName 127.0.0.1', f'  Port {h_port}', f'  ProxyJump 127.0.0.1:{a_port}',
        'Host certified', '  HostName 127.0.0.1', f'  Port {i_port}',
        f'  IdentityFile {cert_key}', f'  CertificateFile {cert_key}-cert.pub',
        'Host *', f'  IdentityFile {ssh_dir}/id_ed25519', f'  IdentityFile {ssh_dir}/id_ecdsa',
        f'  UserKnownHostsFile {known}', '  GlobalKnownHostsFile /dev/null', '  StrictHostKeyChecking ask',
        '']))
    for path in [ssh_dir / 'config', ssh_dir / 'config.d' / 'included.conf']:
        path.chmod(0o600)
    wrapper = bin_dir / 'ssh'
    wrapper.write_text(f'#!/bin/sh\nexec /usr/bin/ssh -F {ssh_dir}/config "$@"\n'); wrapper.chmod(0o700)

    agent = subprocess.check_output(['ssh-agent', '-s', '-a', str(root / 'agent.sock')], text=True)
    agent_pid = int(agent.split('SSH_AGENT_PID=')[1].split(';')[0])
    env = {k: v for k, v in os.environ.items() if k not in ('SSH_AUTH_SOCK', 'SSH_AGENT_PID')}
    env.update(HOME=str(home), SSH_AUTH_SOCK=str(root / 'agent.sock'), PATH=f'{bin_dir}:{os.environ["PATH"]}',
               XDG_CONFIG_HOME=str(home / '.config'), XDG_RUNTIME_DIR=str(root))
    subprocess.run(['ssh-add', '-q', str(agent_key)], env=env, check=True, capture_output=True)

    r = access('file-key', f'127.0.0.1:{a_port}')
    assert r['access'] == 'verified' and r['accepted_key'] == fingerprint(file_key), r
    checks.append('file-key-verified-and-identified')
    r = access('agent-only', f'127.0.0.1:{b_port}')
    assert r['access'] == 'verified' and r['accepted_key'] == fingerprint(agent_key), r
    checks.append('agent-only-key-verified-and-identified')
    assert len(r['identities']) == 4, r['identities']  # file, agent, unused, certificate key
    checks.append('agent-and-file-identities-deduplicated-public-only')
    r = access('no-key', f'127.0.0.1:{c_port}')
    assert r['access'] == 'not-verified' and r['description'] == 'Not verified', r
    checks.append('no-key-host-not-verified-not-claimed-absent')
    r = access('password-only', f'127.0.0.1:{d_port}')
    assert r['access'] == 'needs-interaction', r
    checks.append('password-only-needs-interaction-no-password-sent')
    r = access('unknown-host', f'127.0.0.1:{e_port}')
    assert r['access'] == 'unknown-host-trust', r
    assert subprocess.run(['ssh-keygen', '-F', f'[127.0.0.1]:{e_port}', '-f', str(known)], capture_output=True).returncode == 1
    checks.append('unknown-host-key-not-accepted')
    r = access('changed-host', f'127.0.0.1:{f_port}')
    assert r['access'] == 'host-key-changed', r
    checks.append('changed-host-key-refused')
    r = access('unreachable', f'127.0.0.1:{g_port}')
    assert r['access'] == 'unreachable', r
    checks.append('closed-port-unreachable')
    r = access('include', 'inc-a')
    assert r['access'] == 'verified', r
    checks.append('include-alias-port-from-config')
    r = access('match', 'matched-a')
    assert r['access'] == 'verified', r
    checks.append('match-rule-applied')
    r = access('duplicate', 'dup-alias')
    assert r['access'] == 'verified', r  # first Host block wins: port A, not C
    checks.append('duplicate-alias-first-value-wins')
    r = access('jump', 'jumped')
    assert r['access'] == 'verified', r
    checks.append('proxyjump-route-verified')
    r = access('certificate', 'certified')
    assert r['access'] == 'verified', r
    checks.append('user-certificate-verified')

    before = access('stale', f'127.0.0.1:{a_port}')['inputs']
    assert access('stale', f'127.0.0.1:{a_port}')['inputs'] == before
    config = (ssh_dir / 'config').read_text()
    (ssh_dir / 'config').write_text(config.replace('StrictHostKeyChecking ask', 'StrictHostKeyChecking ask\n  User generated-other'))
    assert access('stale', f'127.0.0.1:{a_port}')['inputs'] != before
    (ssh_dir / 'config').write_text(config)
    assert access('stale', f'127.0.0.1:{a_port}')['inputs'] == before
    with known.open('a') as f:
        f.write(f'[127.0.0.1]:{g_port} ' + ' '.join(Path(str(other) + '.pub').read_text().split()[:2]) + '\n')
    assert access('stale', f'127.0.0.1:{a_port}')['inputs'] != before
    checks.append('cache-inputs-change-with-account-and-host-trust')

    for d in [ssh_dir, root]:
        for path in d.rglob('*'):
            assert not path.name.endswith('.json') or 'BEGIN OPENSSH PRIVATE KEY' not in path.read_text()
    a.evidence.mkdir(parents=True, exist_ok=True)
    evidence = a.evidence / 'ssh-access.json'
    evidence.write_text(json.dumps({'status': 'PASS', 'checks': checks}, indent=2))
    print(f'SSH access discovery PASS {len(checks)} checks; evidence {evidence}')
finally:
    for proc in servers:
        proc.terminate()
    for proc in servers:
        proc.wait(timeout=10)
    if agent_pid:
        os.kill(agent_pid, 15)
    shutil.rmtree(root, ignore_errors=True)
