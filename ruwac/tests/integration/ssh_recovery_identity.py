"""Actual trusted SSH alias redirection: pending work must reject before authentication."""
from pathlib import Path
import argparse,hashlib,json,os,pwd,select,socket,subprocess,sys,tempfile,time,uuid
p=argparse.ArgumentParser();p.add_argument('--driver',type=Path,required=True);p.add_argument('--previous-driver',type=Path,help='Create pending journals with an earlier released adapter');p.add_argument('--server-key-type',choices=['ed25519','rsa','ecdsa'],default='ed25519');p.add_argument('--gui',type=Path);p.add_argument('--binary',type=Path);p.add_argument('--evidence',type=Path,required=True);p.add_argument('--expect-auth-leak',action='store_true',help='Historical reproduction only; never use for qualification');a=p.parse_args()
checkout=Path(__file__).resolve().parents[3];archive=a.evidence.resolve()
if (checkout/'scripts/public-files.py').exists()and(archive==checkout or checkout in archive.parents):p.error('Evidence must stay outside the source checkout')
if os.geteuid()==0 or not Path(f'/run/user/{os.getuid()}').is_dir():p.error('Use a non-root fixture account with its private runtime directory')
a.evidence.mkdir(mode=0o700,parents=True,exist_ok=True)
if a.gui or a.binary:
 if not(a.gui and a.binary)or os.environ.get('WAYLAND_DISPLAY')!='ruwac-test' or os.environ.get('RUWAC_TEST_MUTTER_INPUT')!='1':p.error('Native checks require both applications and an owned test Wayland display')
 os.sched_setaffinity(0,sorted(os.sched_getaffinity(0))[:2])
r=Path(tempfile.mkdtemp(prefix='route-',dir=a.evidence));r.chmod(0o700);auth=Path(tempfile.mkdtemp(prefix='ruwac-recovery-auth-',dir=f'/run/user/{os.getuid()}'));auth.chmod(0o700);children=[];logs=[];checks=[];user=pwd.getpwuid(os.getuid()).pw_name
env={**os.environ,'HOME':str(r/'home'),'XDG_STATE_HOME':str(r/'state'),'PYTHONDONTWRITEBYTECODE':'1'}
for n in ['home','state','bin']:(r/n).mkdir(mode=0o700)
def run(args,**kw):return subprocess.run(list(map(str,args)),env=env,capture_output=True,text=True,timeout=40,**kw)
def wait(f,timeout=6):
 end=time.monotonic()+timeout
 while time.monotonic()<end:
  value=f()
  if value:return value
  time.sleep(.02)
 raise AssertionError('Owned SSH readiness deadline')
def api(action,*args,legacy=False):
 driver=(a.previous_driver or a.driver)if legacy else a.driver
 q=run([driver,action,r/'profile.json',*args]);assert q.returncode==0,q.stderr;return json.loads(q.stdout)
def route(port):
 (r/'ssh_config').write_text(f'Host generated-route\n HostName 127.0.0.1\n Port {port}\n User {user}\n');(r/'ssh_config').chmod(0o600)
def accepted():return (r/'b.log').read_text().count('Accepted publickey')
def reject(action,*args):
 before=accepted();q=api(action,*args);assert not q['ok'] and ('host key changed'in q['error'].lower()or'host identity changed'in q['error'].lower()),q
 time.sleep(.15);count=accepted()-before
 assert (count>0 if a.expect_auth_leak else count==0),{'operation':action,'unexpected_host_authentications':count,'result':q}
 checks.append({'operation':action,'changed_trusted_target_refused':True,'replacement_host_authenticated':count>0});print(action,'replacement authentications',count,flush=True)
try:
 for name in ['client','host-a','host-b']:
  q=run(['ssh-keygen','-q','-t','ed25519'if name=='client'else a.server_key_type,'-N','','-f',r/name]);assert q.returncode==0,q.stderr;(r/name).chmod(0o600)
 authorized=auth/'keys';authorized.write_bytes((r/'client.pub').read_bytes());authorized.chmod(0o600)
 ports=[];known=r/'known_hosts';known.touch(mode=0o600)
 for name in ['a','b']:
  with socket.socket()as s:s.bind(('127.0.0.1',0));port=s.getsockname()[1]
  ports.append(port);key=(r/('host-'+name+'.pub')).read_text().split()
  with known.open('a')as f:f.write(f'[127.0.0.1]:{port} {key[0]} {key[1]}\n')
  cfg=r/(name+'.conf');cfg.write_text(f'ListenAddress 127.0.0.1\nPort {port}\nHostKey {r}/host-{name}\nPidFile {r}/{name}.pid\nAuthorizedKeysFile {authorized}\nStrictModes yes\nPasswordAuthentication no\nKbdInteractiveAuthentication no\nUsePAM no\nPermitRootLogin no\nAllowUsers {user}\nSubsystem sftp internal-sftp\nLogLevel VERBOSE\n')
  log=(r/(name+'.log')).open('w');logs.append(log);child=subprocess.Popen(['/usr/sbin/sshd','-D','-e','-f',str(cfg)],env=env,stdout=log,stderr=log);children.append(child)
  def ready():
   assert child.poll()is None,'Owned SSH daemon ended'
   try:
    with socket.create_connection(('127.0.0.1',port),.1):return True
   except OSError:return False
  wait(ready)
 shim=r/'bin/ssh';shim.write_text('#!/usr/bin/python3\nimport os,sys\nos.execv("/usr/bin/ssh",["ssh","-F",'+repr(str(r/'ssh_config'))+']+sys.argv[1:])\n');shim.chmod(0o700);env['PATH']=str(r/'bin')+':'+env['PATH']
 profile={'id':'endpoint_'+uuid.uuid4().hex,'machine_id':'machine_'+uuid.uuid4().hex,'name':'Generated original host','group':'Generated','protocol':'SSH','server':'generated-route','username':user,'secret_reference':'generated-unused','favorite':False,'last_used':0,'tags':[],'preferences':{'identity_file':str(r/'client'),'known_hosts':str(known)}};(r/'profile.json').write_text(json.dumps(profile));(r/'profile.json').chmod(0o600)
 route(ports[0]);source=r/'source.bin';source.write_bytes(bytes(range(256))*32768);target=r/'target.bin';target.write_bytes(b'original destination')
 q=api('resume-upload',source,target,'overwrite',1048576,legacy=True);assert not q['ok']and'paused'in q['error'],q
 rows=api('pending');assert rows['ok']and len(rows['result'])==1,rows;job=rows['result'][0];part=target.with_name('ruwac-part-'+job['id']);assert 0<part.stat().st_size<source.stat().st_size;before=hashlib.sha256(part.read_bytes()).hexdigest()
 route(ports[1]);reject('resume',job['id']);reject('discard',job['id']);assert hashlib.sha256(part.read_bytes()).hexdigest()==before and target.read_bytes()==b'original destination'
 route(ports[0]);q=api('resume',job['id']);assert q['ok']and target.read_bytes()==source.read_bytes(),q;checks.append({'operation':'restore-original-alias-and-resume','PASS':True})
 # A genuine crashed rsync journal uses the same original server identity.
 dest=r/'rsync-target';dest.write_text('original rsync destination');process=subprocess.Popen([str(a.previous_driver or a.driver),'rsync-upload',str(r/'profile.json'),str(source),str(dest),'overwrite','no-cancel','copy'],env=env,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True);children.append(process)
 assert select.select([process.stdout],[],[],35)[0],'Rsync checkpoint deadline';message=json.loads(process.stdout.readline());assert message.get('checkpoint')=='copy',message;process.kill();process.wait(timeout=5)
 route(ports[1]);reject('rsync-recover',message['id']);assert dest.read_text()=='original rsync destination'
 route(ports[0]);q=api('rsync-recover',message['id']);assert q['ok'],q;checks.append({'operation':'restore-original-alias-and-rsync-recovery','PASS':True})
 # Rotate the listening peer after SFTP preparation while its old channel
 # remains alive. Updating normal trust must not retarget the secondary SSH.
 dest=r/'rotated-rsync-target';dest.write_text('preserved destination')
 process=subprocess.Popen([str(a.driver),'rsync-upload',str(r/'profile.json'),str(source),str(dest),'overwrite','no-cancel','copy'],env=env,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True);children.append(process)
 assert select.select([process.stdout],[],[],35)[0],'Rsync copy checkpoint deadline';message=json.loads(process.stdout.readline());assert message.get('checkpoint')=='copy',message
 pending=api('rsync-pending');assert pending['ok'],pending;recovery=next(j for j in pending['result']if j['id']==message['id'])
 first=children[0];first.terminate();first.wait(timeout=5)
 old=(r/'a.conf').read_text();(r/'rotated.conf').write_text(old.replace(str(r/'host-a'),str(r/'host-b')))
 rotated_log=(r/'rotated.log').open('w');logs.append(rotated_log);rotated=subprocess.Popen(['/usr/sbin/sshd','-D','-e','-f',str(r/'rotated.conf')],env=env,stdout=rotated_log,stderr=rotated_log);children.append(rotated)
 def rotated_ready():
  assert rotated.poll()is None,'Rotated owned server ended'
  try:
   with socket.create_connection(('127.0.0.1',ports[0]),.1):return True
  except OSError:return False
 wait(rotated_ready);saved=known.read_text();key=(r/'host-b.pub').read_text().split();known.write_text(''.join(f'[127.0.0.1]:{port} {key[0]} {key[1]}\n'for port in ports))
 process.send_signal(__import__('signal').SIGCONT);out,err=process.communicate(timeout=90);q=json.loads(out);time.sleep(.1);count=(r/'rotated.log').read_text().count('Accepted publickey')
 assert (q['ok'] and count>0 if a.expect_auth_leak else not q['ok'] and count==0),{'secondary_channel':q,'replacement_authentications':count,'stderr':err}
 if not a.expect_auth_leak:assert dest.read_text()=='preserved destination'
 pending=api('rsync-pending');assert pending['ok']and not any(j['id']==message['id']for j in pending['result']),pending
 assert not Path(recovery['stage']).exists()and not Path(recovery['backup']).exists(),'Automatic rollback left staging/backup behind'
 checks.append({'operation':'rsync-secondary-channel-after-server-and-trust-rotation','replacement_host_authenticated':count>0});print('rsync secondary replacement authentications',count,flush=True)
 rotated.terminate();rotated.wait(timeout=5);known.write_text(saved);log=(r/'a-restored.log').open('w');logs.append(log);restored=subprocess.Popen(['/usr/sbin/sshd','-D','-e','-f',str(r/'a.conf')],env=env,stdout=log,stderr=log);children.append(restored)
 wait(lambda:restored.poll()is None and (r/'a-restored.log').read_text().find('Server listening')>=0)
 q=api('rsync-upload',source,dest,'overwrite');assert q['ok']and dest.read_bytes()==source.read_bytes(),q;checks.append({'operation':'normal-original-host-rsync-after-refusal','PASS':True})
 if a.gui:
  from spike import Client
  sock=r/'control.sock';log=(r/'engine.log').open('w');logs.append(log);engine=subprocess.Popen([str(a.binary),'--socket',str(sock),'serve'],env=env,stdout=log,stderr=log);children.append(engine)
  def engine_ready():
   assert engine.poll()is None,'Owned engine ended'
   try:
    with Client(sock)as c:return c.call('health')['ok']
   except(OSError,EOFError):return None
  wait(engine_ready)
  with Client(sock)as c:terminal=c.call('session.create',{'name':'Generated identity check','argv':['/bin/cat']})
  log=(r/'gui.log').open('w');logs.append(log);gui=subprocess.Popen([str(a.gui),'--socket',str(sock),'--session',terminal['session'],'--fixture-dir',str(r)],env=env,stdout=log,stderr=log);children.append(gui);sequence=0
  def command(action,**kw):
   global sequence
   sequence+=1;temporary=r/'request.tmp';temporary.write_text(json.dumps({'seq':sequence,'action':action,**kw}));temporary.replace(r/'gui-request.json')
   def response():
    assert gui.poll()is None,'Owned GUI ended'
    try:
     q=json.loads((r/'gui-response.json').read_text());return q['result']if q['seq']==sequence else None
    except(OSError,ValueError):return None
   return wait(response,12)
  command('inspect');profile['server']=f'127.0.0.1:{ports[0]}';(r/'profile.json').write_text(json.dumps(profile));native=r/'native-target';native.write_text('native original')
  q=api('resume-upload',source,native,'overwrite',1048576,legacy=True);assert not q['ok']and'paused'in q['error'],q;pending=api('pending');assert pending['ok']and len(pending['result'])==1,pending;job=pending['result'][0];partial=native.with_name('ruwac-part-'+job['id']);partial_hash=hashlib.sha256(partial.read_bytes()).hexdigest()
  command('files-open',profile=profile,path=str(r));wait(lambda:any('Remote directory ready'in text for text in command('files-inspect')['labels']),15)
  restored.terminate();restored.wait(timeout=5);log=(r/'native-replacement.log').open('w');logs.append(log);replacement=subprocess.Popen(['/usr/sbin/sshd','-D','-e','-f',str(r/'rotated.conf')],env=env,stdout=log,stderr=log);children.append(replacement);wait(lambda:replacement.poll()is None and 'Server listening'in(r/'native-replacement.log').read_text());known.write_text(''.join(f'[127.0.0.1]:{port} {key[0]} {key[1]}\n'for port in ports))
  for operation in ['resume','discard']:
   command('files-transfer',operation='open');wait(lambda:len(command('files-transfer')['rows'])==1)
   command('files-transfer',operation=operation,id=job['id']);wait(lambda:not command('files-transfer')['open']);wait(lambda:any('Original SSH host key changed'in text for text in command('files-inspect')['labels']))
   assert 'Accepted publickey'not in(r/'native-replacement.log').read_text();assert native.read_text()=='native original'and hashlib.sha256(partial.read_bytes()).hexdigest()==partial_hash
   checks.append({'operation':'native-'+operation+'-rejects-newly-trusted-server','replacement_host_authenticated':False});print('native',operation,'zero replacement authentications',flush=True)
  replacement.terminate();replacement.wait(timeout=5);known.write_text(saved);log=(r/'native-original.log').open('w');logs.append(log);original=subprocess.Popen(['/usr/sbin/sshd','-D','-e','-f',str(r/'a.conf')],env=env,stdout=log,stderr=log);children.append(original);wait(lambda:original.poll()is None and 'Server listening'in(r/'native-original.log').read_text())
  command('files-transfer',operation='open');wait(lambda:len(command('files-transfer')['rows'])==1);command('files-transfer',operation='resume',id=job['id']);wait(lambda:any('Transfer verified'in text for text in command('files-inspect')['labels']),20);assert native.read_bytes()==source.read_bytes()and not partial.exists();checks.append({'operation':'native-resume-original-server','PASS':True})
  target=r/'native-rsync';assert command('files-rsync',control='files-rsync-upload',local=source.name,name=target.name,overwrite=False)['invoked'];wait(lambda:any('Rsync verified'in text for text in command('files-inspect')['labels']),25);assert target.read_bytes()==source.read_bytes();checks.append({'operation':'native-rsync-original-server','PASS':True})
  with Client(sock)as c:assert c.call('pane.list',{'session':terminal['session']})[0]['pid']==terminal['pid']
 receipt={'PASS':True,'baseline_authentication_leak_expected':a.expect_auth_leak,'checks':checks,'server_key_type':a.server_key_type,'real_trusted_distinct_SSH_servers':True,'source_sha256':hashlib.sha256(source.read_bytes()).hexdigest(),'owner_configuration_or_processes_changed':False,'driver_sha256':hashlib.sha256(a.driver.read_bytes()).hexdigest(),'previous_driver_sha256':hashlib.sha256(a.previous_driver.read_bytes()).hexdigest()if a.previous_driver else None,'application_hashes':{name:hashlib.sha256(path.read_bytes()).hexdigest()for name,path in [('ruwac',a.binary),('ruwac-desktop',a.gui)]if path}};(r/'acceptance.json').write_text(json.dumps(receipt,indent=2)+'\n');print('Historical authentication leak reproduced'if a.expect_auth_leak else'Original SSH identity protection PASS',len(checks),'groups',flush=True)
finally:
 for child in reversed(children):
  if child.poll()is None:child.terminate()
  try:child.wait(timeout=6)
  except subprocess.TimeoutExpired:child.kill();child.wait(timeout=5)
 for log in logs:log.close()
 for f in auth.iterdir():f.unlink()
 auth.rmdir()
