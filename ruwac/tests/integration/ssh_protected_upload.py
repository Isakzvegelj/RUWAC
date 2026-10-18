"""Production protected PNG upload acceptance using generated loopback OpenSSH state."""
from pathlib import Path
import hashlib, signal, uuid
from PIL import Image
import argparse, json, os, pwd, shlex, socket, subprocess, tempfile, time

p=argparse.ArgumentParser(description='Generated real SSH protected PNG delivery and cancellation checks')
p.add_argument('--driver',type=Path,required=True);p.add_argument('--evidence',type=Path,required=True);p.add_argument('--jump',action='store_true',help='Use the production SSH jump-alias ProxyCommand for the original connection')
a=p.parse_args();archive=a.evidence.resolve();production=a.driver.resolve()
source_root=Path(__file__).resolve().parents[3]
if source_root==archive or source_root in archive.parents:p.error('Evidence must stay outside the source checkout')
if os.geteuid()==0 or not Path(f'/run/user/{os.getuid()}').is_dir():p.error('A non-root test account with its own private runtime directory is required')
archive.mkdir(mode=0o700,parents=True,exist_ok=True)
r = Path(tempfile.mkdtemp(prefix='probe-', dir=archive)); r.chmod(0o700)
authroot = Path(tempfile.mkdtemp(prefix='ruwac-route-', dir=f'/run/user/{os.getuid()}')); authroot.chmod(0o700)
children = []; logs = []; checks = []; control = None
env = {k:v for k,v in os.environ.items() if k not in ['SSH_AUTH_SOCK','SSH_ASKPASS','SSH_ASKPASS_REQUIRE']}
env['HOME'] = str(r)
def run(argv, **kw):
    return subprocess.run([str(v) for v in argv], env=env, capture_output=True, timeout=8, **kw)
def eventually(f):
    deadline=time.monotonic()+6
    while time.monotonic()<deadline:
        if f():return
        time.sleep(.03)
    raise AssertionError('Owned SSH readiness deadline')
def check(name):checks.append(name);print(name,'PASS',flush=True)
try:
    for name in ['client','host-a','host-b']+(['host-g']if a.jump else []):
        q=run(['ssh-keygen','-q','-t','ed25519','-N','','-f',r/name]);assert q.returncode==0
        (r/name).chmod(0o600)
    auth=authroot/'authorized_keys'; auth.write_bytes((r/'client.pub').read_bytes());auth.chmod(0o600)
    known=r/'known_hosts';known.touch(mode=0o600)
    endpoints=[]
    for label in ['a','b']+(['g']if a.jump else []):
        with socket.socket() as s:s.bind(('127.0.0.1',0));port=s.getsockname()[1]
        home=r/('remote-'+label);home.mkdir(mode=0o700)
        marker=home/'commands'
        forced=r/('shell-'+label)
        forced.write_text('#!/bin/sh\nexport HOME='+shlex.quote(str(home))+'\nprintf x >> '+shlex.quote(str(marker))+'\nexec /bin/sh -c "$SSH_ORIGINAL_COMMAND"\n');forced.chmod(0o700)
        cfg=r/('sshd-'+label)
        cfg.write_text(f'ListenAddress 127.0.0.1\nPort {port}\nHostKey {r}/host-{label}\nPidFile {r}/sshd-{label}.pid\nAuthorizedKeysFile {auth}\nForceCommand {forced}\nStrictModes yes\nPasswordAuthentication no\nKbdInteractiveAuthentication no\nUsePAM no\nAllowTcpForwarding local\nAllowUsers {pwd.getpwuid(os.getuid()).pw_name}\nLogLevel ERROR\n')
        log=(r/('sshd-'+label+'.log')).open('wb');logs.append(log)
        child=subprocess.Popen(['/usr/sbin/sshd','-D','-e','-f',str(cfg)],env=env,stdout=log,stderr=log);children.append(child)
        def listening():
            assert child.poll() is None,'Owned SSH server exited'
            try:
                with socket.create_connection(('127.0.0.1',port),timeout=.1):return True
            except OSError:return False
        eventually(listening)
        key=(r/('host-'+label+'.pub')).read_text().split()
        with known.open('a') as f:f.write(f'[127.0.0.1]:{port} {key[0]} {key[1]}\n')
        endpoints.append((port,home,marker))
    config=r/'ssh_config'
    def route(port):
        config.write_text(f'Host generated-route\n HostName 127.0.0.1\n Port {port}\n User {pwd.getpwuid(os.getuid()).pw_name}\n IdentityFile {r}/client\n IdentitiesOnly yes\n UserKnownHostsFile {known}\n StrictHostKeyChecking yes\n BatchMode yes\n ConnectTimeout 3\n');config.chmod(0o600)
        if a.jump:
            with config.open('a')as f:f.write(f'Host generated-jump\n HostName 127.0.0.1\n Port {endpoints[2][0]}\n User {pwd.getpwuid(os.getuid()).pw_name}\n IdentityFile {r}/client\n IdentitiesOnly yes\n UserKnownHostsFile {known}\n StrictHostKeyChecking yes\n BatchMode yes\n ConnectTimeout 3\n')
    route(endpoints[0][0])
    base=['ssh','-F',config]
    def channel_reply(*args):
        q=run([production,'ssh-protected-channel-argv',profile,*args])
        assert q.returncode==0,q.stderr.decode()
        return json.loads(q.stdout)
    def driver_args(action,*args):
        assert action=='client'
        reply=channel_reply(*args);assert reply['ok'],reply
        return reply['result']['argv']
    wrappers=r/'bin';wrappers.mkdir()
    wrapper=wrappers/'ssh';wrapper.write_text('#!/usr/bin/python3\nimport os,sys\nif "-W" in sys.argv:\n with open('+repr(str(r/'gateway-pids'))+',"a")as f:f.write(str(os.getpid())+"\\n")\nos.execv("/usr/bin/ssh",["ssh","-F",'+repr(str(config))+']+sys.argv[1:])\n');wrapper.chmod(0o700)
    env['PATH']=str(wrappers)+os.pathsep+env['PATH']
    profile=r/'profile.json';identifier=uuid.uuid4().hex
    profile.write_text(json.dumps({'id':'endpoint_'+identifier,'machine_id':'machine_'+identifier,'name':'Generated protected upload','group':'Generated','protocol':'SSH','server':'generated-route','username':pwd.getpwuid(os.getuid()).pw_name,'secret_reference':'generated-unused','favorite':False,'last_used':0,'tags':[],'preferences':{'identity_file':str(r/'client'),'known_hosts':str(known)}}));profile.chmod(0o600)
    if a.jump:
        value=json.loads(profile.read_text());value['preferences']['ssh_jump_alias']='generated-jump';profile.write_text(json.dumps(value))
    q=run([production,'ssh-protected-argv',profile]);assert q.returncode==0,q.stderr.decode()
    reply=json.loads(q.stdout);assert reply['ok'],reply;prepared=reply['result'];planned=prepared['argv'];token=prepared['transport'];assert token
    planned.insert(planned.index('--'),'-N')
    image=r/'generated.png';Image.new('RGBA',(64,64),(15,100,200,127)).save(image);image.chmod(0o600)
    def png_upload(cancel=None):
        return run([production,'ssh-image-paste-original',profile,image,token,master.pid,*([] if cancel is None else [cancel])])
    def published_png():
        q=png_upload();reply=json.loads(q.stdout);assert q.returncode==0 and reply['ok'],(q.returncode,q.stdout.decode(),q.stderr.decode())
        path=Path(reply['result']['path'])
        assert path.is_relative_to(endpoints[0][1]) and path.read_bytes()==image.read_bytes()
        assert path.stat().st_mode & 0o777==0o600
        assert not list(endpoints[1][1].rglob('*.png'))
        return path

    control=Path(next(a.removeprefix('ControlPath=') for a in planned if a.startswith('ControlPath=')))
    log=(r/'master.log').open('wb');logs.append(log)
    master=subprocess.Popen(planned,env=env,stdout=log,stderr=log);children.append(master)
    eventually(lambda:control.exists())
    assert run([*base,'-S',control,'-O','check','generated-route']).returncode==0
    pinned=driver_args('client',token,str(master.pid))
    q=run([*pinned,'printf "%s" "$HOME"']);assert q.returncode==0 and q.stdout.decode()==str(endpoints[0][1])
    if a.jump:assert (r/'gateway-pids').exists(), 'Original connection bypassed the managed gateway'
    check('initial-command-uses-original-authenticated-connection')
    published_png();check('production-PNG-adapter-writes-verified-private-image-on-original-connection')
    route(endpoints[1][0])
    q=run([*base,'-o','ControlMaster=no','-o','ControlPath=none','generated-route','printf "%s" "$HOME"']);assert q.returncode==0 and q.stdout.decode()==str(endpoints[1][1])
    check('edited-alias-really-points-new-connections-at-different-host')
    payload=b'generated-route-bound-transfer\x00\xff'
    q=run([*pinned,'umask 077; cat > "$HOME/pinned-data"; printf "%s" "$HOME"'],input=payload)
    assert q.returncode==0 and q.stdout.decode()==str(endpoints[0][1])
    assert (endpoints[0][1]/'pinned-data').read_bytes()==payload and not (endpoints[1][1]/'pinned-data').exists()
    check('existing-connection-keeps-upload-on-original-host-after-config-edit')
    published_png();check('production-PNG-upload-stays-on-original-server-after-effective-alias-redirection')
    before=endpoints[1][2].read_bytes()
    config.write_text('GENERATED_INVALID_SSH_CONFIG\n')
    pinned=driver_args('client',token,str(master.pid))
    q=run([*pinned,'printf "%s" "$HOME"'])
    assert q.returncode==0 and q.stdout.decode()==str(endpoints[0][1])
    check('compiled-upload-client-ignores-invalid-current-SSH-configuration')
    published_png();check('production-PNG-upload-ignores-invalid-current-configuration')
    first_shell=r/'shell-a'
    original_script=first_shell.read_text()
    first_shell.write_text(original_script.replace('exec /bin/sh', 'if test -f "$HOME/hold-upload"; then touch "$HOME/upload-started"; sleep 3; fi\nexec /bin/sh'))
    hold=endpoints[0][1]/'hold-upload';hold.touch()
    before_pngs=set(endpoints[0][1].rglob('*.png'))
    q=png_upload(200);assert q.returncode==0 and not json.loads(q.stdout)['ok']
    assert (endpoints[0][1]/'upload-started').exists()
    hold.unlink();time.sleep(3.2)
    assert set(endpoints[0][1].rglob('*.png'))==before_pngs
    published_png();check('cancelled-production-PNG-channel-publishes-nothing-and-preserves-master')
    before=endpoints[1][2].read_bytes()
    master.send_signal(signal.SIGKILL);master.wait(timeout=5)
    q=png_upload();assert q.returncode==0 and not json.loads(q.stdout)['ok'] and endpoints[1][2].read_bytes()==before
    assert set(endpoints[0][1].rglob('*.png'))!=set()  # prior successes remain
    check('production-PNG-upload-refuses-dead-master-without-an-alternate-host-command')
    assert control.exists(),'SIGKILL should leave an owned stale socket fixture'
    reply=channel_reply(token,str(master.pid));assert not reply['ok']
    # Recipe obtained while the original master was alive must still fail
    # safely if that process dies before OpenSSH opens its channel.
    q=run([*pinned,'cat > "$HOME/must-not-exist"'],input=payload)
    assert q.returncode!=0 and endpoints[1][2].read_bytes()==before
    assert not any((e[1]/'must-not-exist').exists() for e in endpoints)
    check('closed-master-refuses-fallback-with-no-new-host-command-or-file')
    control.unlink()
    reply=channel_reply(token,str(master.pid));assert not reply['ok']
    check('compiled-client-refuses-missing-original-socket')
    control.write_bytes(b'generated-regular-file')
    reply=channel_reply(token,str(master.pid));assert not reply['ok']
    assert control.read_bytes()==b'generated-regular-file'
    control.unlink();control.symlink_to(config)
    reply=channel_reply(token,str(master.pid));assert not reply['ok'] and config.read_text()=='GENERATED_INVALID_SSH_CONFIG\n'
    control.unlink()
    check('compiled-client-refuses-file-and-symlink-socket-substitution')
    replacement=socket.socket(socket.AF_UNIX,socket.SOCK_STREAM)
    replacement.bind(str(control));replacement.listen(4)
    try:
        reply=channel_reply(token,str(master.pid));assert not reply['ok']
        assert 'process changed' in reply['error']
    finally:replacement.close();control.unlink()
    check('compiled-client-refuses-another-process-real-Unix-socket-at-original-path')

    if a.jump:assert len((r/'gateway-pids').read_text().splitlines())==1,'Implicit upload opened a new gateway connection'
    receipt={'managed_jump_alias':a.jump,'production_adapter_sha256':hashlib.sha256(production.read_bytes()).hexdigest(),'PNG_sha256':hashlib.sha256(image.read_bytes()).hexdigest(),'checks':checks,'scope':'Actual loopback OpenSSH with generated servers/keys/homes/config and real binary stdin; production profile preparation, original-connection adapter, ready/commit publication, cancellation and socket substitution over actual loopback SSH; no external host/provider or physical input acceptance','protected_owner_configuration_or_processes_changed':False,'test_directory':str(r),'ssh_version':run(['ssh','-V']).stderr.decode().strip()}
    (archive/'protected-ssh-upload.json').write_text(json.dumps(receipt,indent=2)+'\n')
finally:
    for child in reversed(children):
        if child.poll() is None:child.terminate()
        try:child.wait(timeout=5)
        except subprocess.TimeoutExpired:child.kill();child.wait()
    for log in logs:log.close()
    if control is not None:control.unlink(missing_ok=True)
    for f in authroot.iterdir():f.unlink()
    authroot.rmdir()
