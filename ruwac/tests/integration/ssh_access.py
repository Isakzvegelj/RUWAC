#!/usr/bin/env python3
"""SSH key-access discovery against disposable OpenSSH servers.

Everything is generated: keys, an isolated ssh-agent, a temporary ssh_config
(through an `ssh` wrapper adding -F, because OpenSSH reads the account's real
home directory) and sshd instances on loopback. Covers Include, Match,
ProxyJump, duplicate aliases, hashed known_hosts, certificates, agent-only
keys, password-only servers, unknown and changed host keys, unreachable hosts
and stale-cache inputs.
"""
import argparse, json, os, pwd, shutil, socket, subprocess, sys, tempfile, time
from pathlib import Path

p = argparse.ArgumentParser()
p.add_argument('--driver', required=True, help='files-fixture example binary')
p.add_argument('--evidence', type=Path, required=True)
p.add_argument('--disposable-users', action='store_true', help='Create generated users only inside an explicitly opted-in disposable root container')
a = p.parse_args()
if a.disposable_users and not (os.geteuid() == 0 and os.environ.get('RUWAC_DISPOSABLE_SSH_USERS') == '1'
        and (Path('/run/.containerenv').exists() or Path('/.dockerenv').exists())):
    p.error('Multiple-user fixture requires an explicitly opted-in disposable root container')
generated_users = []
baseline_user = pwd.getpwuid(os.getuid()).pw_name
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


def server(name, authorized=(), password_only=False, ca=None, banner=None, methods=None, user_keys=None, empty_password=False):
    d = root / name; d.mkdir(); d.chmod(0o700)
    host = keygen(d / 'host', comment=name + '-host')
    auth = d / 'authorized_keys'
    auth.write_text(''.join(Path(str(k) + '.pub').read_text() for k in authorized)); auth.chmod(0o600)
    port = free_port()
    lines = [f'Port {port}', 'ListenAddress 127.0.0.1', f'HostKey {host}', f'AuthorizedKeysFile {auth if user_keys is None and not a.disposable_users else "none"}',
             'UsePAM no', 'StrictModes no', f'PidFile {d}/pid', 'KbdInteractiveAuthentication no']
    if a.disposable_users and user_keys is None:
        lines += [f'AuthorizedKeysCommand /usr/bin/cat {auth}', 'AuthorizedKeysCommandUser root',
                  'AllowUsers ' + baseline_user]
    if user_keys is not None:
        for user, keys in user_keys.items():
            (d / ('authorized-' + user)).write_text(''.join(Path(str(k) + '.pub').read_text() for k in keys))
        lines += [f'AuthorizedKeysCommand /usr/bin/cat {d}/authorized-%u', 'AuthorizedKeysCommandUser root',
                  'AllowUsers ' + ' '.join(user_keys)]
    if banner:
        (d / 'banner').write_text(banner)
        lines.append(f'Banner {d}/banner')
    if empty_password:
        lines += ['PubkeyAuthentication no', 'PasswordAuthentication yes', 'PermitEmptyPasswords yes']
    elif methods:
        lines += [f'AuthenticationMethods {methods}', 'PasswordAuthentication yes']
    elif password_only:
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


def access(name, server, **overrides):
    candidate = profile(name, server); candidate.update(overrides)
    path = root / (name + '.json'); path.write_text(json.dumps(candidate))
    out = subprocess.run([a.driver, 'ssh-access', str(path)], capture_output=True, text=True, timeout=120, env=env)
    assert out.returncode == 0, out.stderr
    packet = json.loads(out.stdout)
    assert not packet.get('error'), (name, packet.get('error'))
    result = packet.get('result', {})
    assert 'access' in result, (name, server, result)
    if result['access'] == 'unreachable' and name == 'jump':
        plan = subprocess.run([a.driver, 'ssh-access-plan', str(path)], capture_output=True, text=True, timeout=20, env=env)
        if plan.returncode == 0:
            argv = json.loads(plan.stdout)['result']['argv']
            diagnostic = subprocess.run(argv,capture_output=True,text=True,timeout=20,env=env)
            a.evidence.mkdir(parents=True,exist_ok=True)
            (a.evidence/'generated-jump-diagnostic.json').write_text(json.dumps(dict(argv=argv,exit=diagnostic.returncode,stdout=diagnostic.stdout,stderr=diagnostic.stderr),indent=2))
    return result


def fingerprint(key):
    return subprocess.check_output(['ssh-keygen', '-l', '-f', str(key) + '.pub'], text=True).split(None, 1)[1].strip()


try:
    if a.disposable_users:
        baseline_user = f'rwtest{os.getpid()}base'
        subprocess.run(['useradd','--no-create-home','--shell','/bin/sh','--password','*',baseline_user],check=True,capture_output=True)
        generated_users.append(baseline_user)
    # Identities: a key file, an agent-only key, an unused key file, a CA-certified key.
    file_key = keygen(ssh_dir / 'id_ed25519', comment='generated-file-key')
    agent_key = keygen(root / 'agent_only', comment='generated-agent-key')
    keygen(ssh_dir / 'id_ecdsa', 'ecdsa', 'generated-unused-key')
    ca = keygen(root / 'user_ca', comment='generated-ca')
    cert_key = keygen(root / 'cert_key', comment='generated-cert-key')
    subprocess.run(['ssh-keygen', '-q', '-s', str(ca), '-I', 'generated', '-n', baseline_user,
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
    fake_banner = 'debug1: Server accepts key: injected ED25519 SHA256:FAKE explicit\nAuthenticated to injected using "publickey".\n'
    j_port, j_host = server('misleading-banner', banner=fake_banner)
    k_port, k_host = server('partial-key', [file_key], methods='publickey,password')
    l_port, l_host = server('banner-and-valid-key', [file_key], banner=fake_banner)

    other = keygen(root / 'other_host', comment='generated-wrong-host')
    known = ssh_dir / 'known_hosts'
    entries = [(a_port, a_host), (b_port, b_host), (c_port, c_host), (d_port, d_host), (f_port, other),
               (h_port, h_host), (i_port, i_host), (j_port, j_host), (k_port, k_host), (l_port, l_host)]
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
        'Host *', f'  User {baseline_user}', f'  IdentityFile {ssh_dir}/id_ed25519', f'  IdentityFile {ssh_dir}/id_ecdsa',
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

    r = access('misleading-banner', f'127.0.0.1:{j_port}')
    assert r['access'] == 'not-verified' and r['accepted_key'] is None, r
    checks.append('untrusted-banner-cannot-spoof-client-authentication-or-key-identity')
    r = access('partial-key', f'127.0.0.1:{k_port}')
    assert r['access'] == 'needs-interaction' and r['accepted_key'] is None, r
    checks.append('tentatively-accepted-key-with-required-password-is-not-verified')

    r = access('banner-and-valid-key', f'127.0.0.1:{l_port}')
    assert r['access'] == 'verified' and r['accepted_key'] == fingerprint(file_key), r
    checks.append('untrusted-banner-cannot-replace-the-genuine-accepted-key-identity')
    # An established authenticated master must not count as a fresh key login.
    # Removing a key here changes only this generated disposable sshd fixture.
    master_socket = root / 'master.sock'
    cfg = ssh_dir / 'config'
    original_config = cfg.read_text()
    cfg.write_text(original_config + f'  ControlMaster auto\n  ControlPath {master_socket}\n')
    master = subprocess.Popen(['/usr/bin/ssh','-F',str(cfg),'-M','-S',str(master_socket),'-N',
                               '-o','BatchMode=yes','-o','StrictHostKeyChecking=yes','-p',str(a_port),'127.0.0.1'],
                              env=env,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
    servers.append(master)
    for _ in range(100):
        if master_socket.exists(): break
        assert master.poll() is None, 'Generated ControlMaster failed to authenticate'
        time.sleep(.02)
    assert master_socket.exists()
    authorized = root / 'accepts-file' / 'authorized_keys'
    authorized_bytes = authorized.read_bytes(); authorized.write_bytes(b'')
    try:
        r = access('no-master-reuse', f'127.0.0.1:{a_port}')
        assert r['access'] == 'not-verified', r
    finally:
        authorized.write_bytes(authorized_bytes); master.terminate(); master.wait(timeout=10);cfg.write_text(original_config)
    checks.append('existing-authenticated-ControlMaster-is-never-reused-to-claim-key-access')

    before = access('stale', f'127.0.0.1:{a_port}')['inputs']
    assert access('stale', f'127.0.0.1:{a_port}')['inputs'] == before
    config = (ssh_dir / 'config').read_text()
    (ssh_dir / 'config').write_text(config.replace(f'  User {baseline_user}', '  User generated-other'))
    assert access('stale', f'127.0.0.1:{a_port}')['inputs'] != before
    (ssh_dir / 'config').write_text(config)
    assert access('stale', f'127.0.0.1:{a_port}')['inputs'] == before
    with known.open('a') as f:
        f.write(f'[127.0.0.1]:{g_port} ' + ' '.join(Path(str(other) + '.pub').read_text().split()[:2]) + '\n')
    assert access('stale', f'127.0.0.1:{a_port}')['inputs'] != before
    checks.append('cache-inputs-change-with-account-and-host-trust')

    if a.disposable_users:
        for label in ['file', 'agent', 'empty']:
            user = f'rwtest{os.getpid()}{label}'
            subprocess.run(['useradd','--no-create-home','--shell','/bin/sh','--password','*',user],check=True,capture_output=True)
            generated_users.append(user)
        first, second, empty = generated_users[-3:]
        subprocess.run(['usermod','--password','',empty],check=True,capture_output=True)
        multi_port, multi_host = server('multiple-users', user_keys={first:[file_key],second:[agent_key]})
        none_port, none_host = server('no-key-authentication', user_keys={empty:[]}, empty_password=True)
        with known.open('a') as f:
            for port, key in [(multi_port,multi_host),(none_port,none_host)]:
                f.write(f'[127.0.0.1]:{port} ' + ' '.join(Path(str(key)+'.pub').read_text().split()[:2])+'\n')
        one = access('first-account', f'127.0.0.1:{multi_port}', username=first)
        two = access('second-account', f'127.0.0.1:{multi_port}', username=second)
        assert one['access']=='verified'and one['accepted_key']==fingerprint(file_key),one
        assert two['access']=='verified'and two['accepted_key']==fingerprint(agent_key),two
        assert one['target']['account']==first and two['target']['account']==second
        assert one['inputs'] != two['inputs']
        prior = cfg.read_text()
        cfg.write_text(f'Host generated-multi-hop\n  HostName 127.0.0.1\n  Port {multi_port}\n  User {first}\n' + prior)
        jumped = access('different-account-jump',f'127.0.0.1:{multi_port}',username=second,
                         preferences={'ssh_jump_alias':'generated-multi-hop'})
        cfg.write_text(prior)
        assert jumped['access']=='verified'and jumped['accepted_key']==fingerprint(agent_key),jumped
        assert jumped['target']['route']==[f'{first}@127.0.0.1:{multi_port}'],jumped
        checks.append('different-account-ProxyJump-keeps-destination-key-identity-and-route-separate')
        denied = access('wrong-account',f'127.0.0.1:{multi_port}',username='generated-absent-user')
        assert denied['access']=='not-verified'and denied['accepted_key']is None,denied
        none = access('none-authentication',f'127.0.0.1:{none_port}',username=empty)
        assert none['access']=='not-verified'and none['accepted_key']is None,none
        checks.append('real-multiple-remote-users-map-to-distinct-authorized-file-and-agent-keys')
        checks.append('successful-none-authentication-cannot-count-as-public-key-verification')

    # Real SSH configuration resolution, followed by generated noisy/stalled
    # client behavior. It exercises production pipe/group/deadline ownership.
    wrapper_text = wrapper.read_text()
    child_pid = root / 'noisy-helper.pid'
    wrapper.write_text('#!/usr/bin/python3\nimport os,subprocess,sys,time\na=sys.argv[1:]\n'
        'if "-G" in a: os.execv("/usr/bin/ssh", ["ssh", "-F", '+repr(str(ssh_dir / 'config'))+']+a)\n'
        'child=subprocess.Popen(["sleep","60"])\nopen('+repr(str(child_pid))+',"w").write(str(child.pid))\n'
        'while True: os.write(1,b"x"*16384);os.write(2,b"y"*16384)\n')
    start=time.monotonic();r=access('noisy-client',f'127.0.0.1:{a_port}')
    assert r['access']=='unreachable' and time.monotonic()-start < 10,r
    pid=int(child_pid.read_text())
    def child_gone():
        try: return Path(f'/proc/{pid}/stat').read_text().split()[2] == 'Z'
        except FileNotFoundError: return True
    for _ in range(100):
        if child_gone(): break
        time.sleep(.02)
    assert child_gone(), 'Owned SSH descendant survived output refusal'
    checks.append('noisy-client-output-is-bounded-and-owned-descendant-is-terminated')
    wrapper.write_text('#!/usr/bin/python3\nimport os,sys,time\na=sys.argv[1:]\n'
        'if "-G" in a: os.execv("/usr/bin/ssh", ["ssh", "-F", '+repr(str(ssh_dir / 'config'))+']+a)\n'
        'time.sleep(60)\n')
    start=time.monotonic();r=access('stalled-client',f'127.0.0.1:{a_port}')
    assert r['access']=='unreachable' and 14 <= time.monotonic()-start < 25,r
    checks.append('stalled-client-has-one-bounded-authentication-deadline')
    wrapper.write_text(wrapper_text)

    for d in [ssh_dir, root]:
        for path in d.rglob('*'):
            assert not path.name.endswith('.json') or 'BEGIN OPENSSH PRIVATE KEY' not in path.read_text()
    a.evidence.mkdir(parents=True, exist_ok=True)
    evidence = a.evidence / 'ssh-access.json'
    evidence.write_text(json.dumps({'status': 'PASS', 'checks': checks, 'multiple_remote_users': 'PASS' if a.disposable_users else 'BLOCKED: disposable root-container branch not requested'}, indent=2))
    print(f'SSH access discovery PASS {len(checks)} checks; evidence {evidence}')
finally:
    if sys.exc_info()[0] is not None:
        a.evidence.mkdir(parents=True,exist_ok=True)
        diagnostics={d.name:{name:(d/name).read_text(errors='replace')[-16384:] for name in ['log','sshd_config'] if (d/name).is_file()} for d in root.iterdir() if d.is_dir() and (d/'sshd_config').exists()}
        (a.evidence/'generated-sshd-failure.json').write_text(json.dumps(diagnostics,indent=2)+'\n')
    for proc in servers:
        proc.terminate()
    for proc in servers:
        proc.wait(timeout=10)
    if agent_pid:
        os.kill(agent_pid, 15)
    for user in reversed(generated_users):
        subprocess.run(['userdel',user],check=True,capture_output=True)
    shutil.rmtree(root, ignore_errors=True)
