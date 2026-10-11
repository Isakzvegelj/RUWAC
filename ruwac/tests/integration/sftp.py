#!/usr/bin/env python3
"""Production libssh2 SFTP against an isolated loopback sshd; generated keys/data only."""
import argparse,hashlib,json,os,pwd,socket,subprocess,tempfile,time,traceback,threading,signal
from pathlib import Path
from PIL import Image
p=argparse.ArgumentParser();p.add_argument('--driver',required=True);p.add_argument('--evidence',required=True);p.add_argument('--gui');p.add_argument('--binary');a=p.parse_args();root=Path(tempfile.mkdtemp(prefix='sftp-',dir=a.evidence));root.chmod(0o700);remote=root/'remote';remote.mkdir(mode=0o700);results=[];server=None
for name in ['host','client']:subprocess.run(['ssh-keygen','-q','-t','ed25519','-N','','-f',str(root/name)],check=True)
(root/'authorized_keys').write_text((root/'client.pub').read_text());(root/'authorized_keys').chmod(0o600)
with socket.socket() as s:s.bind(('127.0.0.1',0));port=s.getsockname()[1]
config=root/'sshd.conf';config.write_text(f'ListenAddress 127.0.0.1\nPort {port}\nHostKey {root}/host\nPidFile {root}/sshd.pid\nAuthorizedKeysFile {root}/authorized_keys\nStrictModes yes\nPasswordAuthentication no\nKbdInteractiveAuthentication no\nUsePAM no\nPermitRootLogin no\nAllowUsers {pwd.getpwuid(os.getuid()).pw_name}\nSubsystem sftp internal-sftp\nLogLevel ERROR\n')
key=(root/'host.pub').read_text().split();known=root/'known_hosts';known.write_text(f'[127.0.0.1]:{port} {key[0]} {key[1]}\n');known.chmod(0o600)
profile={'id':'generated-endpoint','machine_id':'generated-machine','name':'fixture','group':'R5','protocol':'SSH','server':f'127.0.0.1:{port}','username':pwd.getpwuid(os.getuid()).pw_name,'secret_reference':'generated-unused','favorite':False,'last_used':0,'tags':[],'preferences':{'identity_file':str(root/'client'),'known_hosts':str(known)}}
profile_path=root/'profile.json';profile_path.write_text(json.dumps(profile));profile_path.chmod(0o600)
def invoke(action,local,remote=None,overwrite=False,cancel=None):
 args=[a.driver,action,str(profile_path),str(local)]
 if remote is not None:args.extend([str(remote),'overwrite' if overwrite else 'skip']);
 if cancel is not None:args.append(str(cancel))
 return json.loads(subprocess.check_output(args,text=True,timeout=30))
def case(name,fn):
 start=time.perf_counter()
 try:detail=fn();results.append({'name':name,'status':'PASS','elapsed_ms':(time.perf_counter()-start)*1000,'detail':detail})
 except Exception as e:results.append({'name':name,'status':'FAIL','error':str(e),'traceback':traceback.format_exc()})
 print(name,results[-1]['status'],flush=True)
try:
 log=(root/'sshd.log').open('wb');server=subprocess.Popen(['/usr/sbin/sshd','-D','-e','-f',str(config)],stdout=log,stderr=log,executable='/usr/sbin/sshd',start_new_session=True)
 for _ in range(100):
  if server.poll() is not None:raise RuntimeError('isolated sshd failed; inspect private fixture log')
  try:
   with socket.create_connection(('127.0.0.1',port),.1):break
  except OSError:time.sleep(.05)
 def integrity():
  folder=remote/'nested directory ž';folder.mkdir();samples={'text ž; $(literal).txt':'Generated fixture Łódź 🦀\n'.encode(),'binary "quotes".bin':bytes(range(256))*4096};png=root/'generated.png';Image.new('RGBA',(65,7),(60,70,80,90)).save(png);samples['image.png']=png.read_bytes();records=[]
  for name,data in samples.items():
   source=root/name;source.write_bytes(data);dest=folder/name;result=invoke('upload',source,dest);assert result['ok'],result;hash=hashlib.sha256(data).hexdigest();assert result['result']['sha256']==hash and hashlib.sha256(dest.read_bytes()).hexdigest()==hash
   download=root/('download-'+name);result=invoke('download',download,dest);assert result['ok'] and hashlib.sha256(download.read_bytes()).hexdigest()==hash,result
   assert not invoke('upload',source,dest)['ok'];r=invoke('upload',source,dest,True);assert r['ok'],r;assert not invoke('download',download,dest)['ok'];r=invoke('download',download,dest,True);assert r['ok'],r;records.append({'bytes':len(data),'sha256':hash,'directions':2})
  listing=invoke('list',folder);assert listing['ok'] and set(listing['result']['entries'])==set(samples),listing
  return records
 case('SFTP-INTEGRITY-AND-OVERWRITE',integrity)
 def guards():
  source=root/'guard.txt';source.write_text('generated');link=remote/'symlink';link.symlink_to(source);assert not invoke('upload',source,link,True)['ok'];assert not invoke('download',root/'guard-copy',link)['ok'];assert not invoke('upload',source,remote/'../escape')['ok']
  deny=remote/'read-only';deny.mkdir(mode=0o500);assert not invoke('upload',source,deny/'file')['ok'];deny.chmod(0o700)
  original=known.read_text();known.write_text('');r=invoke('list',remote);assert not r['ok'] and 'host key' in r['error'].lower(),r;known.write_text(original)
  key=(root/'client.pub').read_text().split();known.write_text(f'[127.0.0.1]:{port} {key[0]} {key[1]}\n');r=invoke('list',remote);assert not r['ok'] and 'CHANGED' in r['error'],r;known.write_text(original)
  return {'symlink':True,'parent_traversal':True,'permission_error':True,'unknown_and_changed_host_key':True}
 case('SFTP-GUARDS',guards)
 def cancel():
  source=root/'large.bin';source.write_bytes(bytes(16*1024*1024));r=invoke('upload',source,remote/'cancel.bin',cancel=60);assert not r['ok'],r;assert not (remote/'cancel.bin').exists();assert not list(remote.glob('ruwac-part*'));return {'cancelled':True,'partial_removed':True}
 case('SFTP-CANCEL',cancel)
 if a.gui and a.binary:
  def native_panel():
   import importlib.util
   spec=importlib.util.spec_from_file_location('spike',Path(__file__).with_name('spike.py'));spike=importlib.util.module_from_spec(spec);spec.loader.exec_module(spike)
   home=root/'home';home.mkdir(mode=0o700);env={**os.environ,'HOME':str(home),'XDG_CONFIG_HOME':str(home/'config'),'GTK_A11Y':'none','GTK_USE_PORTAL':'0'}
   sock=root/'daemon.sock';daemon=subprocess.Popen([a.binary,'--socket',str(sock),'serve'],env=env,stdout=(root/'daemon.log').open('wb'),stderr=subprocess.STDOUT);gui=None
   try:
    spike.eventually(lambda:sock.exists())
    with spike.Client(sock)as client:session=client.call('session.create',{'name':'Generated SFTP workspace'})
    gui=subprocess.Popen([a.gui,'--socket',str(sock),'--session',session['session'],'--fixture-dir',str(root)],env=env,stdout=(root/'gui.log').open('wb'),stderr=subprocess.STDOUT)
    seq=0
    def command(action,**kw):
     nonlocal seq
     seq+=1;temp=root/'gui-request.tmp';temp.write_text(json.dumps({'seq':seq,'action':action,**kw}));temp.replace(root/'gui-request.json')
     def result():
      try:r=json.loads((root/'gui-response.json').read_text());return r['result']if r['seq']==seq else None
      except (ValueError,FileNotFoundError):return None
     return spike.eventually(result,10)
    command('inspect');command('files-open',profile=profile,path=str(remote/'nested directory ž'))
    r=spike.eventually(lambda:r if 'Remote directory ready' in (r:=command('files-inspect'))['labels'] else False,10)
    assert any('text ž; $(literal).txt' in label for label in r['buttons']),r
    command('files-inspect',path=str(remote/'missing-directory'),refresh=True)
    spike.eventually(lambda:any('failed' in s.lower() for s in command('files-inspect')['labels']),10)
    with spike.Client(sock)as client:pid=client.call('pane.list',{'session':session['session']})[0]['pid']
    command('tab-select',tab=0);tree=command('mixed-join',tab=1,vertical=False);assert len(tree['tabs'])==1,tree
    assert command('views-manage')['clicked']
    moved=command('views-action',operation='move',index=1);assert moved['clicked'] and len(moved['layout']['tabs'])==2,moved
    command('files-inspect',path=str(remote/'nested directory ž'),refresh=True)
    r=spike.eventually(lambda:r if 'Remote directory ready' in (r:=command('files-inspect'))['labels'] else False,10)
    assert any('text ž; $(literal).txt' in label for label in r['buttons']),r
    assert command('views-manage')['clicked']
    closed=command('views-action',operation='close',index=0);assert closed['clicked'] and len(closed['layout']['tabs'])==1 and closed['terminal_views']==1,closed
    with spike.Client(sock)as client:assert client.call('pane.list',{'session':session['session']})[0]['pid']==pid
    return {'native_in_window_panel':True,'actual_async_directory_list':True,'permissions_and_names_visible':True,'recoverable_error':True,'native_mixed_files_move_and_refresh':True,'native_files_close_preserved_terminal_pid':True,'transfer_chooser':'compiled native dialogs; byte operations qualified separately'}
   finally:
    for proc in [gui,daemon]:
     if proc and proc.poll()is None:proc.terminate();proc.wait(timeout=5)
  case('SFTP-NATIVE-PANEL',native_panel)
 def slow_peer():
  listener=socket.socket();listener.bind(('127.0.0.1',0));listener.listen(1)
  def stall():
   connection,_=listener.accept();time.sleep(6);connection.close();listener.close()
  thread=threading.Thread(target=stall,daemon=True);thread.start();original=profile['server'];profile['server']=f'127.0.0.1:{listener.getsockname()[1]}';profile_path.write_text(json.dumps(profile));start=time.perf_counter();r=invoke('list',remote);elapsed=(time.perf_counter()-start)*1000;assert not r['ok'] and elapsed<7500,r;profile['server']=original;profile_path.write_text(json.dumps(profile));return {'timeout_ms':elapsed,'bounded_connection_error':True}
 case('SFTP-SLOW-PEER',slow_peer)
 def disconnect():
  source=root/'disconnect-large.bin'
  with source.open('wb')as file:file.truncate(512*1024*1024)
  destination=remote/'disconnect-target';destination.write_bytes(b'original-generated-file')
  child=subprocess.Popen([a.driver,'upload',str(profile_path),str(source),str(destination),'overwrite'],stdout=subprocess.PIPE,text=True)
  try:
   time.sleep(.15)
   parents={}
   for proc in Path('/proc').iterdir():
    if not proc.name.isdigit():continue
    try:parents[int(proc.name)]=int(next(line.split()[1]for line in (proc/'status').read_text().splitlines()if line.startswith('PPid:')))
    except (FileNotFoundError,ProcessLookupError,PermissionError,StopIteration):pass
   descendants=set();frontier={server.pid}
   while frontier:
    found={pid for pid,parent in parents.items()if parent in frontier};descendants.update(found);frontier=found
   for pid in descendants:
    try:os.kill(pid,signal.SIGTERM)
    except ProcessLookupError:pass
   os.killpg(server.pid,signal.SIGTERM);server.wait(timeout=5);r=json.loads(child.communicate(timeout=20)[0]);assert not r['ok'],r;assert destination.read_bytes()==b'original-generated-file'
   r=invoke('list',remote);assert not r['ok'],r;partials=list(remote.glob('ruwac-part*'));return {'mid_transfer_transport_loss':True,'original_preserved':True,'visible_error':True,'partials_after_lost_transport':len(partials),'cleanup':'best effort; remote partial may remain when the transport is gone'}
  finally:
   if child.poll()is None:child.terminate();child.wait(timeout=5)
 case('SFTP-DISCONNECT',disconnect)
finally:
 if server and server.poll() is None:server.terminate();server.wait(timeout=5)
 report={'cases':results,'fixture':'isolated current-user loopback sshd, generated ed25519 identity and host keys','source':'production libssh2 adapter','limitations':['No rsync','No external SSH host','Remote namespace mutation by another actor is not an atomic SFTP snapshot']};(root/'receipt.json').write_text(json.dumps(report,indent=2));Path(a.evidence,'sftp-acceptance.json').write_text(json.dumps(report,indent=2));print('Evidence',root)
raise SystemExit(0 if len(results)==(6 if a.gui and a.binary else 5) and all(r['status']=='PASS'for r in results)else 1)
