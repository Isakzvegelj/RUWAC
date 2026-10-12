#!/usr/bin/env python3
"""Real production rsync/SFTP against an owned SSH fixture; optional native callbacks."""
import argparse,hashlib,json,os,pwd,signal,socket,stat,subprocess,tempfile,time,traceback
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('--driver',required=True);p.add_argument('--evidence',required=True);p.add_argument('--gui');p.add_argument('--binary');a=p.parse_args();root=Path(tempfile.mkdtemp(prefix='rsync-files-',dir=a.evidence)).resolve();root.chmod(0o700);remote=root/'remote [literal] * ?';remote.mkdir(mode=0o700);results=[];server=None
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
try:
 log=(root/'sshd.log').open('wb');server=subprocess.Popen(['/usr/sbin/sshd','-D','-e','-f',str(config)],stdout=log,stderr=log,start_new_session=True)
 for _ in range(100):
  if server.poll()is not None:raise RuntimeError('owned SSH fixture failed')
  try:
   with socket.create_connection(('127.0.0.1',port),.1):break
  except OSError:time.sleep(.05)
 def trees():
  source=root/'source tree [x]*? ž';source.mkdir();(source/'empty').mkdir();(source/'nested "quotes"').mkdir();
  for name,data in [('space and \'quote.txt',b'generated UTF-8 \xc5\xbe'),('literal [x]*?.bin',bytes(range(256))*4096),('$(literal); & file',b'shell metacharacters remain literal')]:
   path=source/'nested "quotes"'/name;path.write_bytes(data);path.chmod(0o640)
  (source/'nested "quotes"').chmod(0o750);(source/'empty').chmod(0o500);source.chmod(0o750)
  for path in [source,*source.rglob('*')]:os.utime(path,(1700000000,1700000000))
  expected=snapshot(source);target=remote/'destination [x]*? with \'quote';first=invoke(True,source,target);assert first['ok'],first;assert snapshot(target)==expected
  local=root/'download tree';second=invoke(False,local,target);assert second['ok'],second;assert snapshot(local)==expected;assert first['result']['sha256']==second['result']['sha256']
  assert not invoke(True,source,target)['ok'];assert not invoke(False,local,target)['ok']
  (target/'extra old file').write_text('old destination');q=invoke(True,source,target,True);assert q['ok'] and snapshot(target)==expected,q
  (local/'extra old file').write_text('old destination');q=invoke(False,local,target,True);assert q['ok'] and snapshot(local)==expected,q
  return{'literal_metacharacters':True,'empty_and_readonly_directories':True,'POSIX_modes':True,'SHA256_tree':first['result']['sha256'],'explicit_replacement':True,'files':first['result']['files']}
 case('RSYNC-TREE-INTEGRITY-PERMISSIONS-REPLACEMENT',trees)
 def files():
  source=root/'single source';source.write_bytes(bytes(range(256))*1000);source.chmod(0o600);target=remote/'single renamed';assert invoke(True,source,target)['ok'];local=root/'single downloaded';assert invoke(False,local,target)['ok'];assert source.read_bytes()==local.read_bytes();return{'rename_and_bidirectional_single_file':True}
 case('RSYNC-SINGLE-FILE',files)
 def guards():
  source=root/'guard source';source.write_text('generated');linked=root/'linked source';linked.symlink_to(source);target=remote/'guard target';q=invoke(True,linked,target);assert not q['ok'] and'symlink'in q['error'].lower(),q;assert not target.exists()
  folder=root/'symlink tree';folder.mkdir();(folder/'link').symlink_to(source);assert not invoke(True,folder,target)['ok'];assert not target.exists();r=remote/'linked directory';r.symlink_to(folder,target_is_directory=True);assert not invoke(True,source,r/'file')['ok']
  sparse=root/'oversize';sparse.touch();sparse.write_bytes(b'');os.truncate(sparse,32*1024**3+1);q=invoke(True,sparse,target);assert not q['ok'] and'32 GiB'in q['error'],q;assert not target.exists()
  trusted=known.read_text();known.write_text('');q=invoke(True,source,target);assert not q['ok'];known.write_text(trusted)
  public=identity.with_name(identity.name+'.pub').read_text().split();known.write_text(f'[127.0.0.1]:{port} {public[0]} {public[1]}\n');assert not invoke(True,source,target)['ok'];known.write_text(trusted)
  return{'source_and_ancestor_symlinks_refused':True,'oversize_refused':True,'unknown_changed_host_keys_refused':True}
 case('RSYNC-PATH-HOST-AND-BUDGET-GUARDS',guards)
 def cancel():
  source=root/'large sparse source';source.touch();os.truncate(source,1024**3);target=remote/'cancel destination';target.write_text('original');q=invoke(True,source,target,True,32768);assert not q['ok'] and'cancel'in q['error'].lower(),q;assert target.read_text()=='original';assert not list(remote.glob('ruwac-rsync-part*'));assert not list(remote.glob('ruwac-rsync-backup*'));return{'actual_running_transfer_cancelled':True,'original_destination_preserved':True,'owned_staging_removed':True}
 case('RSYNC-CANCEL-PRESERVES-DESTINATION',cancel)
 def missing_remote():
  with socket.socket()as probe:probe.bind(('127.0.0.1',0));new_port=probe.getsockname()[1]
  wrapper=root/'remote-capability-fixture';wrapper.write_text('#!/usr/bin/python3\nprint("rsync  version 2.6.9 protocol version 29")\n');wrapper.chmod(0o700)
  c=root/'limited-sshd.conf';c.write_text(config.read_text().replace(f'Port {port}',f'Port {new_port}')+'ForceCommand '+str(wrapper)+'\n')
  f=(root/'limited-sshd.log').open('wb');limited=subprocess.Popen(['/usr/sbin/sshd','-D','-e','-f',str(c)],stdout=f,stderr=f,start_new_session=True)
  original=profile['server'];trust=known.read_text()
  try:
   for _ in range(100):
    try:
     with socket.create_connection(('127.0.0.1',new_port),.1):break
    except OSError:time.sleep(.02)
   profile['server']=f'127.0.0.1:{new_port}';profile_file.write_text(json.dumps(profile));known.write_text(trust+f'[127.0.0.1]:{new_port} {key[0]} {key[1]}\n')
   q=invoke(True,root/'guard source',remote/'missing-rsync-destination');assert not q['ok'] and'3.4'in q['error'],q;assert not(remote/'missing-rsync-destination').exists();return{'real_SSH_capability_probe':True,'unsupported_remote_version_refused_before_staging':True,'fixture_scope':'Actual SSH with fixed generated forced-command capability response; not a real legacy rsync executable'}
  finally:
   profile['server']=original;profile_file.write_text(json.dumps(profile));known.write_text(trust);os.killpg(limited.pid,signal.SIGTERM);limited.wait(timeout=5);f.close()
 case('RSYNC-REMOTE-CAPABILITY-GATE',missing_remote)
 if a.gui and a.binary:
  def native():
   import spike
   env={**os.environ,'XDG_CONFIG_HOME':str(root/'config'),'XDG_DATA_HOME':str(root/'data'),'XDG_CACHE_HOME':str(root/'cache'),'XDG_STATE_HOME':str(root/'state'),'HOME':str(root/'home'),'GSETTINGS_BACKEND':'memory'}
   for folder in ['config','data','cache','state','home']:(root/folder).mkdir(mode=0o700,exist_ok=True)
   sock=root/'control.sock';children=[]
   def start(args,name):
    f=(root/(name+'.log')).open('wb');process=subprocess.Popen(args,env=env,stdout=f,stderr=f);children.append((process,f));return process
   seq=0
   def command(action,**kw):
    nonlocal seq
    seq+=1;temp=root/'request.tmp';temp.write_text(json.dumps({'seq':seq,'action':action,**kw}));temp.replace(root/'gui-request.json');deadline=time.monotonic()+10
    while time.monotonic()<deadline:
     try:d=json.loads((root/'gui-response.json').read_text())
     except(OSError,ValueError):time.sleep(.02);continue
     if d['seq']==seq:return d['result']
     time.sleep(.02)
    raise AssertionError('Native callback response timeout')
   try:
    start([a.binary,'--socket',str(sock),'serve'],'daemon');spike.eventually(lambda:sock.exists())
    with spike.Client(str(sock))as client:session=client.call('session.create',{'name':'Owned rsync native fixture'});pid=client.call('pane.list',{'session':session['session']})[0]['pid']
    gui=start([a.gui,'--new-instance','--clipboard-disabled','--socket',str(sock),'--fixture-dir',str(root)],'gui');spike.eventually(lambda:sock.with_suffix('.gui.sock').exists(),20)
    q=command('files-open',profile=profile,path=str(remote));assert'error'not in q,q
    spike.eventually(lambda:'Remote directory ready'in command('files-inspect')['labels'],20)
    source=root/'native source';source.mkdir();(source/'item ž [x]*?.txt').write_text('native generated fixture')
    result=command('files-rsync',control='files-rsync-folder',local='native source',name='native target',overwrite=False);assert result['invoked'],result
    deadline=time.monotonic()+30
    while time.monotonic()<deadline:
     state=command('files-inspect')
     if any(label.startswith('Rsync verified')for label in state['labels']):break
     time.sleep(.05)
    else:raise AssertionError(state)
    assert snapshot(source)==snapshot(remote/'native target')
    assert any(b=='Rsync download…'for b in state['buttons']);assert gui.poll()is None
    with spike.Client(str(sock))as client:assert client.call('pane.list',{'session':session['session']})[0]['pid']==pid
    target=remote/'native cancellation';target.write_text('original native destination');assert command('files-rsync',control='files-rsync-upload',local='large sparse source',name=target.name,overwrite=True)['invoked']
    def active_transport():
     for task in Path(f'/proc/{gui.pid}/task').iterdir():
      try:children=(task/'children').read_text().split()
      except OSError:continue
      for pid in children:
       try:args=Path(f'/proc/{pid}/cmdline').read_bytes().split(b'\0')
       except OSError:continue
       if b'--secluded-args'in args:return int(pid)
     return None
    transport=spike.eventually(active_transport,20);assert command('files-transfer',operation='cancel')is not None
    deadline=time.monotonic()+15
    while time.monotonic()<deadline:
     state=command('files-inspect')
     if any('Rsync cancelled'in label for label in state['labels']):break
     time.sleep(.05)
    else:raise AssertionError(state)
    assert target.read_text()=='original native destination';assert not list(remote.glob('ruwac-rsync-part*'));assert not Path(f'/proc/{transport}').exists()
    command('snapshot');return{'real_native_transport_cancelled':True,'original_destination_preserved':True,'real_in_window_rsync':True,'actual_native_async_callback':True,'terminal_PID_preserved':True,'physical_input':False,'native_file_chooser':'Compiled; selected-path callback exercised synthetically'}
   finally:
    for process,f in reversed(children):
     if process.poll()is None:process.terminate()
     try:process.wait(timeout=6)
     except subprocess.TimeoutExpired:process.kill();process.wait(timeout=3)
     f.close()
  case('RSYNC-NATIVE-FILES-PANEL',native)
finally:
 if server and server.poll()is None:os.killpg(server.pid,signal.SIGTERM);server.wait(timeout=5)
 try:authorized.unlink();auth.rmdir()
 except OSError:pass
 receipt={'results':results,'root':str(root),'driver_sha256':hashlib.sha256(Path(a.driver).read_bytes()).hexdigest(),'artifacts':{name:hashlib.sha256(Path(path).read_bytes()).hexdigest()for name,path in [('files-fixture',a.driver),('ruwac',a.binary),('ruwac-desktop',a.gui)]if path},'scope':'Production rsync/OpenSSH/SFTP against an owned actual loopback SSH server; generated files/keys only; optional GTK on isolated compositor with synthetic callbacks, not physical input'}
 (root/'receipt.json').write_text(json.dumps(receipt,indent=2)+'\n');print(json.dumps(receipt,indent=2))
if not results or any(v['status']!='PASS'for v in results):raise SystemExit(1)
