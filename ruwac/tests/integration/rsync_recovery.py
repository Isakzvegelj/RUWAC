#!/usr/bin/env python3
"""Real production rsync/SFTP against an owned SSH fixture; optional native callbacks."""
import argparse,hashlib,json,os,pwd,signal,socket,stat,subprocess,tempfile,time,traceback
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('--driver',required=True);p.add_argument('--evidence',required=True);p.add_argument('--gui');p.add_argument('--binary');a=p.parse_args();root=Path(tempfile.mkdtemp(prefix='rsync-recovery-',dir=a.evidence)).resolve();root.chmod(0o700);remote=root/'remote [literal] * ?';remote.mkdir(mode=0o700);results=[];server=None
identity=root/'client key with space and \'quote';host=root/'host'
for path in [identity,host]:
 subprocess.run(['ssh-keygen','-q','-t','ed25519','-N','','-f',str(path)],check=True);path.chmod(0o600)
auth=Path(tempfile.mkdtemp(prefix='ruwac-rsync-auth-',dir=Path('/run/user')/str(os.getuid())));auth.chmod(0o700);authorized=auth/'authorized_keys';authorized.write_bytes(identity.with_name(identity.name+'.pub').read_bytes());authorized.chmod(0o600)
with socket.socket()as sock:sock.bind(('127.0.0.1',0));port=sock.getsockname()[1]
config=root/'sshd.conf';config.write_text(f'ListenAddress 127.0.0.1\nPort {port}\nHostKey {host}\nPidFile {root}/sshd.pid\nAuthorizedKeysFile {authorized}\nStrictModes yes\nPasswordAuthentication no\nKbdInteractiveAuthentication no\nUsePAM no\nPermitRootLogin no\nAllowUsers {pwd.getpwuid(os.getuid()).pw_name}\nSubsystem sftp internal-sftp\nLogLevel ERROR\n')
key=host.with_suffix('.pub').read_text().split();known=root/'known hosts \'quotes';known.write_text(f'[127.0.0.1]:{port} {key[0]} {key[1]}\n');known.chmod(0o600)
profile={'id':'generated-endpoint','machine_id':'generated-machine','name':'Rsync fixture','group':'Files','protocol':'SSH','server':f'127.0.0.1:{port}','username':pwd.getpwuid(os.getuid()).pw_name,'secret_reference':'generated-unused','favorite':False,'last_used':0,'tags':[],'preferences':{'identity_file':str(identity),'known_hosts':str(known)}}
profile_file=root/'profile.json';profile_file.write_text(json.dumps(profile));profile_file.chmod(0o600)
def invoke(upload,local,target,overwrite=False,cancel=None):
 args=[a.driver,'rsync-upload'if upload else'rsync-download',str(profile_file),str(local),str(target),'overwrite'if overwrite else'skip']
 if cancel is not None:args.append(str(cancel))
 return json.loads(subprocess.check_output(args,env={**os.environ,'XDG_STATE_HOME':str(root/'state')},text=True,timeout=120))
def case(name,fn):
 start=time.monotonic()
 try:detail=fn();results.append({'name':name,'status':'PASS','elapsed_ms':round((time.monotonic()-start)*1000,1),'detail':detail})
 except Exception as e:results.append({'name':name,'status':'FAIL','error':str(e),'traceback':traceback.format_exc()})
 print(name,results[-1]['status'],flush=True)
def snapshot(path):
 paths=[path]+list(path.rglob('*'))if path.is_dir()else[path]
 return{str(p.relative_to(path)): {'mode':stat.S_IMODE(p.stat().st_mode),'type':'directory'if p.is_dir()else'file','mtime_seconds':int(p.stat().st_mtime),'sha256':None if p.is_dir()else hashlib.sha256(p.read_bytes()).hexdigest()}for p in paths}
import select
state=root/'state';state.mkdir(mode=0o700,exist_ok=True);env={**os.environ,'XDG_STATE_HOME':str(state)}
def api(action,*args):return json.loads(subprocess.check_output([a.driver,action,str(profile_file),*map(str,args)],env=env,text=True,timeout=120))
spawned=[]
def crash(up,local,target,phase):
 process=subprocess.Popen([a.driver,'rsync-upload'if up else'rsync-download',str(profile_file),str(local),str(target),'overwrite','no-cancel',phase],env=env,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True)
 spawned.append(process)
 ready,_,_=select.select([process.stdout],[],[],35)
 if not ready:process.kill();process.wait();raise AssertionError('Checkpoint not reached')
 line=process.stdout.readline();message=json.loads(line);assert message.get('checkpoint')==phase,message
 return process,message['id']
def kill(process):process.kill();process.wait(timeout=8)
def jobs():
 q=api('rsync-pending');assert q['ok'],q;return q['result']
try:
 log=(root/'sshd.log').open('wb');server=subprocess.Popen(['/usr/sbin/sshd','-D','-e','-f',str(config)],stdout=log,stderr=log,start_new_session=True)
 for _ in range(100):
  if server.poll()is not None:raise RuntimeError('owned SSH fixture failed')
  try:
   with socket.create_connection(('127.0.0.1',port),.1):break
  except OSError:time.sleep(.05)
 def boundaries():
  records=[]
  for up in [True,False]:
   for directory in [False,True]:
    for phase in ['copy','backup-intent','backup-moved','published','cleaned']:
     name=f'{up}-{directory}-{phase}';source=root/('source-'+name);destination=root/('destination-'+name)
     if directory:
      source.mkdir();(source/'new.bin').write_bytes(b'new generated tree');(source/'empty').mkdir();destination.mkdir();(destination/'original.bin').write_bytes(b'original generated tree')
     else:source.write_bytes(b'new generated file');destination.write_bytes(b'original generated file')
     original=snapshot(destination);expected=snapshot(source)
     process,id=crash(up,source if up else destination,destination if up else source,phase)
     # A stopped yet live transfer still owns its lock; recovery cannot race it.
     q=api('rsync-recover',id);assert not q['ok'] and'active'in q['error'],q
     kill(process);q=api('rsync-recover',id);assert q['ok'],q
     assert snapshot(destination)==(expected if phase in ['published','cleaned']else original)
     assert jobs()==[] and not list(state.rglob('*.files'))
     records.append({'upload':up,'directory':directory,'phase':phase,'lock_conflict_refused':True,'SIGKILL_recovery':True})
  return records
 case('RSYNC-RECOVERY-20-REAL-KILL-BOUNDARIES',boundaries)
 def inflight():
  source=root/'inflight source';source.touch();os.truncate(source,1024**3);target=remote/'inflight target';target.write_text('original inflight')
  process,id=crash(True,source,target,'copy');process.send_signal(signal.SIGCONT)
  deadline=time.monotonic()+20;child=None
  while time.monotonic()<deadline and process.poll()is None:
   for task in Path(f'/proc/{process.pid}/task').iterdir():
    try:children=(task/'children').read_text().split()
    except OSError:continue
    for pid in children:
     try:args=Path(f'/proc/{pid}/cmdline').read_bytes().split(b'\0')
     except OSError:continue
     if b'--secluded-args'in args:child=int(pid);break
   if child:break
   time.sleep(.005)
  assert child is not None,'Actual rsync transport child must be live at crash'
  kill(process);q=api('rsync-recover',id);assert q['ok']and target.read_text()=='original inflight',q;assert jobs()==[]
  return{'SIGKILL_during_actual_rsync_child':True,'original_preserved':True,'owned_staging_filelist_journal_removed':True}
 case('RSYNC-RECOVERY-INFLIGHT-CLIENT-CRASH',inflight)
 def tampering():
  source=root/'tamper-source';source.write_text('new');destination=root/'tamper-target';destination.write_text('original');process,id=crash(True,source,destination,'published');kill(process)
  job=next(j for j in jobs()if j['id']==id);backup=Path(job['backup']);stage=Path(job['stage']);marker=stage/'.ruwac-rsync-owner';original_marker=marker.read_bytes();marker.write_text('different owner');q=api('rsync-recover',id);assert not q['ok'];assert destination.read_text()=='new' and backup.read_text()=='original'
  marker.write_bytes(original_marker);stage.chmod(0o755);q=api('rsync-recover',id);assert not q['ok'];assert backup.read_text()=='original';stage.chmod(0o700)
  destination.write_text('independent actor');q=api('rsync-recover',id);assert not q['ok'] and'changed'in q['error'],q;assert backup.read_text()=='original';destination.write_text('new')
  original_profile=profile['username'];profile['username']='stale-user';profile_file.write_text(json.dumps(profile));q=api('rsync-recover',id);assert not q['ok'] and'profile changed'in q['error'],q;profile['username']=original_profile;profile_file.write_text(json.dumps(profile))
  path=state/'RUWAC/rsync-recovery'/f'{id}.json';saved=path.read_bytes();altered=json.loads(saved);altered['stage']=str(root/'unrelated');path.write_text(json.dumps(altered));q=api('rsync-recover',id);assert not q['ok'];assert backup.read_text()=='original';path.write_bytes(saved)
  q=api('rsync-recover',id);assert q['ok'] and jobs()==[],q
  return{'changed_marker_stage_permissions_destination_profile_and_journal_refused':True,'original_backup_preserved_until_verified_recovery':True}
 case('RSYNC-RECOVERY-STALE-TAMPER-GUARDS',tampering)
 def normal_actor_changes():
  records=[]
  for phase in ['backup-moved','published']:
   source=root/('normal-source-'+phase);source.write_text('new expected');target=remote/('normal-target-'+phase);target.write_text('original expected');process,id=crash(True,source,target,phase)
   job=next(j for j in jobs()if j['id']==id);backup=Path(job['backup'])
   changed=backup if phase=='backup-moved'else target;changed.write_text('independent actor data');process.send_signal(signal.SIGCONT);output=process.communicate(timeout=120)[0];q=json.loads(output.strip());assert not q['ok']and'changed'in q['error'],q;assert changed.read_text()=='independent actor data';assert backup.exists()
   if phase=='backup-moved':backup.write_text('original expected');stamp=job['original_stamp'];os.utime(backup,(stamp['mtime'],stamp['mtime']))
   else:target.write_text('new expected')
   q=api('rsync-recover',id);assert q['ok'],q;assert jobs()==[];records.append({'phase':phase,'resumed_normal_operation_refused_changed_data':True,'backup_preserved':True})
  return records
 case('RSYNC-RECOVERY-NORMAL-COMMIT-ACTOR-GUARDS',normal_actor_changes)
 def changed_host():
  global server
  source=root/'host-source';source.write_text('new host fixture');target=remote/'host-target';target.write_text('original host fixture');process,id=crash(True,source,target,'backup-moved');kill(process);job=next(j for j in jobs()if j['id']==id)
  original_config=config.read_text();original_trust=known.read_text();replacement=root/'replacement-host';subprocess.run(['ssh-keygen','-q','-t','ed25519','-N','','-f',str(replacement)],check=True);replacement.chmod(0o600)
  def restart(text):
   global server
   if server.poll()is None:os.killpg(server.pid,signal.SIGTERM);server.wait(timeout=5)
   config.write_text(text);server=subprocess.Popen(['/usr/sbin/sshd','-D','-e','-f',str(config)],stdout=log,stderr=log,start_new_session=True)
   for _ in range(100):
    try:
     with socket.create_connection(('127.0.0.1',port),.1):return
    except OSError:time.sleep(.05)
   raise AssertionError('Rekeyed fixture unavailable')
  try:
   restart(original_config.replace(str(host),str(replacement)));key2=replacement.with_suffix('.pub').read_text().split();known.write_text(f'[127.0.0.1]:{port} {key2[0]} {key2[1]}\n')
   q=api('rsync-recover',id);assert not q['ok'] and'host key changed'in q['error'].lower(),q;assert not target.exists()and Path(job['backup']).read_text()=='original host fixture'
  finally:restart(original_config);known.write_text(original_trust)
  q=api('rsync-recover',id);assert q['ok']and target.read_text()=='original host fixture',q
  return{'actual_SSH_host_key_replacement':True,'updated_trust_did_not_retarget_original_job':True,'recovery_after_original_host_restored':True}
 case('RSYNC-RECOVERY-ORIGINAL-SSH-HOST-IDENTITY',changed_host)
 if a.gui and a.binary:
  def native():
   import spike
   source=root/'native recovery source';source.write_text('new native');target=remote/'native recovery target';target.write_text('original native');process,id=crash(True,source,target,'backup-moved');kill(process);assert not target.exists()
   sock=root/'native.sock';children=[];native_env={**env,'XDG_CONFIG_HOME':str(root/'config'),'XDG_DATA_HOME':str(root/'data'),'XDG_CACHE_HOME':str(root/'cache'),'HOME':str(root/'home'),'GSETTINGS_BACKEND':'memory'}
   for folder in ['config','data','cache','home']:(root/folder).mkdir(mode=0o700,exist_ok=True)
   def start(args,name):
    f=(root/(name+'.log')).open('wb');p=subprocess.Popen(args,env=native_env,stdout=f,stderr=f);children.append((p,f));return p
   seq=0
   def command(action,**kw):
    nonlocal seq
    seq+=1;temp=root/'request.tmp';temp.write_text(json.dumps({'seq':seq,'action':action,**kw}));temp.replace(root/'gui-request.json');deadline=time.monotonic()+10
    while time.monotonic()<deadline:
     try:q=json.loads((root/'gui-response.json').read_text())
     except(OSError,ValueError):time.sleep(.02);continue
     if q['seq']==seq:return q['result']
     time.sleep(.02)
    raise AssertionError('Native response timeout')
   try:
    start([a.binary,'--socket',str(sock),'serve'],'daemon');spike.eventually(lambda:sock.exists())
    with spike.Client(str(sock))as client:session=client.call('session.create',{'name':'Owned native recovery'});pid=client.call('pane.list',{'session':session['session']})[0]['pid']
    gui=start([a.gui,'--socket',str(sock),'--clipboard-disabled','--new-instance','--fixture-dir',str(root)],'gui');spike.eventually(lambda:sock.with_suffix('.gui.sock').exists(),20)
    command('files-open',profile=profile,path=str(remote));spike.eventually(lambda:'Remote directory ready'in command('files-inspect')['labels'],20)
    command('files-rsync-recovery',operation='open');spike.eventually(lambda:len(command('files-rsync-recovery')['rows'])==1,20)
    command('files-rsync-recovery',operation='recover',id=id)
    spike.eventually(lambda:any(s.startswith('Rsync recovery complete')for s in command('files-inspect')['labels']),20)
    assert target.read_text()=='original native' and jobs()==[]
    with spike.Client(str(sock))as client:assert client.call('pane.list',{'session':session['session']})[0]['pid']==pid
    return{'real_native_recovery_dialog_and_callback':True,'actual_original_backup_restored':True,'terminal_PID_preserved':True,'physical_input':False}
   finally:
    for p,f in reversed(children):
     if p.poll()is None:p.terminate()
     try:p.wait(timeout=6)
     except subprocess.TimeoutExpired:p.kill();p.wait(timeout=3)
     f.close()
  case('RSYNC-RECOVERY-NATIVE-DIALOG',native)

finally:
 for process in spawned:
  if process.poll()is None:kill(process)
 if server and server.poll()is None:os.killpg(server.pid,signal.SIGTERM);server.wait(timeout=5)
 try:authorized.unlink();auth.rmdir()
 except OSError:pass
 receipt={'results':results,'root':str(root),'artifacts':{name:hashlib.sha256(Path(path).read_bytes()).hexdigest()for name,path in [('files-fixture',a.driver),('ruwac',a.binary),('ruwac-desktop',a.gui)]if path},'scope':'Actual SIGKILL of production transfer adapter at observed durable transition boundaries; owned loopback SSH/generated files only; not remote power-loss or physical input'}
 (root/'receipt.json').write_text(json.dumps(receipt,indent=2)+'\n');print(json.dumps(receipt,indent=2))
if not results or any(v['status']!='PASS'for v in results):raise SystemExit(1)
