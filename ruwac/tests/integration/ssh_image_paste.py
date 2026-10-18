#!/usr/bin/env python3
"""Real OpenSSH, independent GDK PNG ownership and generated raw PTY consumers.

Requires an explicitly owned Mutter compositor. No provider request, external
account, owner clipboard, home configuration or host service is changed.
"""
import argparse, hashlib, json, os, pwd, shlex, shutil, socket, stat
import subprocess, sys, tempfile, time, uuid
from pathlib import Path
from PIL import Image
from spike import Client, eventually

p=argparse.ArgumentParser()
for flag in ['binary','gui','driver']:p.add_argument('--'+flag,required=True)
p.add_argument('--evidence',type=Path,required=True);p.add_argument('--launch-context',nargs='?',const='agent',choices=['agent','ssh']);a=p.parse_args()
assert os.environ.get('WAYLAND_DISPLAY','').startswith('ruwac-')
assert os.environ.get('RUWAC_TEST_MUTTER_INPUT')=='1'
root=Path(tempfile.mkdtemp(prefix='ssh-image-paste-',dir=a.evidence));root.chmod(0o700)
env={**os.environ,'GTK_A11Y':'none','GSETTINGS_BACKEND':'memory','GTK_USE_PORTAL':'0'}
for key,name in [('HOME','home'),('XDG_CONFIG_HOME','config'),('XDG_CACHE_HOME','cache'),('XDG_DATA_HOME','data')]:
 env[key]=str(root/name);Path(env[key]).mkdir(mode=0o700)
for key in ['RUWAC_SOCKET','RUWAC_KEYMAP','RUWAC_SHORTCUTS','TMUX','STY','BASH_ENV','ENV','SSH_ASKPASS','RUWAC_ASKPASS_REF','SSH_AUTH_SOCK']:env.pop(key,None)
wrappers=root/'bin';wrappers.mkdir()
ssh_config=root/'ssh_config';ssh_config.write_text('');ssh_config.chmod(0o600)
wrapper=wrappers/'ssh';wrapper.write_text('#!/usr/bin/python3\nimport os,sys\nos.execv("/usr/bin/ssh",["ssh","-F",'+repr(str(ssh_config))+']+sys.argv[1:])\n');wrapper.chmod(0o700)
env['PATH']=str(wrappers)+os.pathsep+env['PATH']
remote=root/'remote home';remote.mkdir(mode=0o700)
started=root/'upload-started';hold=root/'hold-upload'
forced=root/'remote-shell';forced.write_text('#!/bin/sh\nexport HOME='+shlex.quote(str(remote))+'\nif test -f '+shlex.quote(str(hold))+'; then touch '+shlex.quote(str(started))+'; n=0; while test -f '+shlex.quote(str(hold))+' && test "$n" -lt 150; do sleep .1; n=$((n+1)); done; fi\nexec /bin/sh -c "$SSH_ORIGINAL_COMMAND"\n');forced.chmod(0o700)
for name in ['host','client','wrong-host']:
 subprocess.run(['ssh-keygen','-q','-t','ed25519','-N','','-f',str(root/name)],check=True)
 (root/name).chmod(0o600)
auth_root=Path(tempfile.mkdtemp(prefix='ruwac-image-auth-',dir=Path('/run/user')/str(os.getuid())));auth_root.chmod(0o700)
auth=auth_root/'authorized_keys';auth.write_text((root/'client.pub').read_text());auth.chmod(0o600)
with socket.socket()as s:s.bind(('127.0.0.1',0));port=s.getsockname()[1]
config=root/'sshd.conf';config.write_text(f'ListenAddress 127.0.0.1\nPort {port}\nHostKey {root}/host\nPidFile {root}/sshd.pid\nAuthorizedKeysFile {auth}\nForceCommand {forced}\nStrictModes yes\nPasswordAuthentication no\nKbdInteractiveAuthentication no\nUsePAM no\nAllowUsers {pwd.getpwuid(os.getuid()).pw_name}\nLogLevel ERROR\n')
known=root/'known_hosts'
def trust(name):
 key=(root/(name+'.pub')).read_text().split();known.write_text(f'[127.0.0.1]:{port} {key[0]} {key[1]}\n');known.chmod(0o600)
trust('host')
identity=uuid.uuid4().hex
profile={'id':'endpoint_'+identity,'machine_id':'machine_'+identity,'name':'Generated image receiver','group':'Generated','protocol':'SSH','server':f'127.0.0.1:{port}','username':pwd.getpwuid(os.getuid()).pw_name,'secret_reference':'generated-unused','favorite':False,'last_used':0,'tags':[],'preferences':{'identity_file':str(root/'client'),'known_hosts':str(known)}}
profile_file=root/'profile.json';profile_file.write_text(json.dumps(profile));profile_file.chmod(0o600)
library=Path(env['XDG_CONFIG_HOME'])/'RUWAC/machines.json';library.parent.mkdir(mode=0o700);library.write_text(json.dumps({'schema':1,'items':[profile],'groups':[]}));library.chmod(0o600)
image=root/'generated.png';Image.frombytes('RGBA',(1024,768),os.urandom(1024*768*4)).save(image);image.chmod(0o600)
digest=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
children=[];checks=[];seq={};sock=root/'control.sock'
def start(argv,name):
 log=(root/(name+'.log')).open('w');child=subprocess.Popen([str(x)for x in argv],env=env,stdout=log,stderr=log);children.append((child,log));return child
def rpc(method,params=None):
 with Client(sock)as client:return client.call(method,params)
def command(who,action,**kw):
 seq[who]=seq.get(who,0)+1;n=seq[who];tmp=root/(who+'-request.tmp');tmp.write_text(json.dumps({'seq':n,'action':action,**kw}));tmp.replace(root/(who+'-request.json'))
 def response():
  try:d=json.loads((root/(who+'-response.json')).read_text());return d['result']if d['seq']==n else None
  except (OSError,ValueError):return None
 return eventually(response,12)
def upload(cancel=None):
 args=[a.driver,'ssh-image-paste',str(profile_file),str(image)]+([]if cancel is None else[str(cancel)])
 q=subprocess.run(args,env=env,capture_output=True,text=True,timeout=15)
 return q,json.loads(q.stdout)
def record(name):checks.append(name);print(name,'PASS',flush=True)
try:
 result=subprocess.run([a.gui,'Are you sure (yes/no)?'],env={**env,'RUWAC_ASKPASS_REF':'generated-unused','RUWAC_ASKPASS_ONLY_SAVED':'1'},capture_output=True,timeout=3)
 assert result.returncode==1 and not result.stdout
 record('background-credential-helper-refuses-host-confirmation-without-a-dialog')
 server=start(['/usr/sbin/sshd','-D','-e','-f',config],'sshd')
 def listening():
  assert server.poll()is None
  with socket.socket()as s:return s.connect_ex(('127.0.0.1',port))==0
 eventually(listening)
 q,out=upload();assert q.returncode==0 and out['ok'],(q.stderr,out)
 path=Path(out['result']['path']);assert digest(path)==digest(image)
 assert stat.S_IMODE(path.stat().st_mode)==0o600 and stat.S_IMODE(path.parent.stat().st_mode)==0o700
 assert not list(path.parent.glob('.upload-*'))
 record('authenticated-OpenSSH-PNG-integrity-private-permissions-atomic-publication')
 before=set(path.parent.iterdir());trust('wrong-host');q,out=upload();assert not out['ok'];trust('host');assert set(path.parent.iterdir())==before
 record('changed-host-key-refused-without-publication')
 target=path.parent;kept=target.with_name('preserved-images');target.rename(kept);outside=root/'unrelated';outside.mkdir();target.symlink_to(outside,target_is_directory=True)
 q,out=upload();assert not out['ok'] and not list(outside.iterdir());target.unlink();kept.rename(target)
 target.chmod(0o755);q,out=upload();assert not out['ok'] and stat.S_IMODE(target.stat().st_mode)==0o755;target.chmod(0o700)
 record('symlink-and-nonprivate-destination-refused-without-changing-existing-metadata')
 hold.touch();started.unlink(missing_ok=True);before=set(target.glob('*.png'));begin=time.monotonic();q,out=upload(150);assert not out['ok'] and time.monotonic()-begin<2
 hold.unlink();time.sleep(3.2);assert set(target.glob('*.png'))==before
 record('cancelled-real-SSH-upload-returns-without-partial-image-publication')

 daemon=start([a.binary,'--socket',sock,'serve'],'daemon');eventually(sock.exists)
 receiver=root/'receiver.py';receiver.write_text('import os,sys,tty\ntty.setraw(0)\nprint("GENERATED_RECEIVER_READY",flush=True)\nwith open(sys.argv[1],"ab",buffering=0)as f:\n while True:\n  b=os.read(0,4096)\n  if not b:break\n  f.write(b)\n')
 def receiver_path(i):return root/('received-'+('decoy'if a.launch_context and i==0 else str(i)))
 sessions=[rpc('session.create',{'name':('Generated codex-named ordinary shell'if a.launch_context and i==0 else 'Generated '+str(i)),'argv':[sys.executable,str(receiver),str(receiver_path(i)) ]})for i in range(2)]
 for s in sessions:rpc('pane.wait',{'pane':s['pane'],'contains':'GENERATED_RECEIVER_READY','timeout_ms':5000})
 if a.launch_context:
  profile['preferences'].update(codex_command=shlex.join([sys.executable,str(receiver),str(root/'received-0')]),agent_backend='direct')
  if a.launch_context=='ssh':
   forced.write_text(forced.read_text().replace('exec /bin/sh -c', 'if test -z "$SSH_ORIGINAL_COMMAND"; then exec '+shlex.join([sys.executable,str(receiver),str(root/'received-0')])+'; fi\nexec /bin/sh -c'))
  profile_file.write_text(json.dumps(profile));library.write_text(json.dumps({'schema':1,'items':[profile],'groups':[]}))
 gui=start([a.gui,'--new-instance','--socket',sock,'--session',sessions[0]['session'],'--session',sessions[1]['session'],'--fixture-dir',root],'gui')
 peer=start([sys.executable,Path(__file__).resolve().parents[1]/'fixtures/desktop_clipboard.py',root],'peer')
 eventually(lambda:(root/'peer-ready.json').exists());command('gui','tab-select',tab=0);eventually(lambda:command('gui','inspect').get('target'),20)
 from gi.repository import Gio,GLib
 bus=Gio.bus_get_sync(Gio.BusType.SESSION,None);dest='org.gnome.Mutter.RemoteDesktop'
 remote_session=bus.call_sync(dest,'/org/gnome/Mutter/RemoteDesktop',dest,'CreateSession',None,None,Gio.DBusCallFlags.NONE,5000,None).unpack()[0]
 def notify(method,args=None):return bus.call_sync(dest,remote_session,dest+'.Session',method,args,None,Gio.DBusCallFlags.NONE,5000,None)
 notify('Start')
 def key(n,state):notify('NotifyKeyboardKeysym',GLib.Variant('(ub)',(n,state)))
 def focus(who):
  for _ in range(12):
   before=command(who,'input-probe')['events']
   key(65493,True);key(65493,False) # F24 is consumed by fixture-only GDK capture.
   time.sleep(.06)
   if command(who,'input-probe')['events']>before:return
   for n,state in [(65513,True),(65289,True),(65289,False),(65513,False)]:key(n,state)
   time.sleep(.2)
  raise AssertionError('Owned synthetic keyboard focus did not reach '+who)
 command('gui','tab-select',tab=0)
 if a.launch_context:
  # Real saved agent action -> real OpenSSH PTY. No synthetic context binding.
  command('gui','host-shortcuts',click='machine-terminal'if a.launch_context=='ssh'else'machine-codex')
  def launched():
   rows=command('gui','launch-contexts')['entries']
   return next((e for e in rows if e['provider']==(None if a.launch_context=='ssh'else'codex')),None)
  context=eventually(launched,12)
  pane=next(p for p in rpc('pane.list',{'session':context['session']})if p['id']==context['pane'])
  sessions[0]={'session':context['session'],'pane':pane['id'],'pid':pane['pid'],'window':pane['window']}
  rpc('pane.wait',{'pane':pane['id'],'contains':'GENERATED_RECEIVER_READY','timeout_ms':5000})
  catalog=Path(env['XDG_CONFIG_HOME'])/'RUWAC/launches'
  def saved_record():
   files=list(catalog.glob('*/'+pane['id']+'.json'))
   return files[0] if files else None
  saved=eventually(saved_record)
  metadata=saved.read_text();assert profile['secret_reference']not in metadata and profile['server']not in metadata
  record('production-saved-action-records-explicit-private-pane-and-provider-over-real-SSH')
  command('gui','workspace-close');gui.wait(timeout=10)
  (root/'gui-request.json').unlink(missing_ok=True);(root/'gui-response.json').unlink(missing_ok=True)
  gui=start([a.gui,'--new-instance','--window-id','window_'+uuid.uuid4().hex,'--socket',sock,'--session',sessions[0]['session'],'--session',sessions[1]['session'],'--fixture-dir',root],'gui-context-restored')
  eventually(lambda:any(e['pane']==pane['id']for e in command('gui','launch-contexts')['entries']),15)
  command('gui','tab-select',tab=0);eventually(lambda:command('gui','inspect').get('target')==pane['id'])
  assert rpc('pane.list',{'session':context['session']})[0]['pid']==pane['pid']
  command('gui','host-shortcuts',operation='header')
  eventually(lambda:sum(x['name']=='agent-session'for x in command('gui','host-shortcuts')['widgets'])==(0 if a.launch_context=='ssh'else 1))
  command('gui','host-shortcuts',close=True)
  record('frontend-restart-restores-live-SSH-image-route-and-agent-menu-without-name-guess-or-new-process')
 else:
  binding=command('gui','ssh-image-paste',profile=profile);assert 'error'not in binding,binding
 received=lambda i:(root/('received-'+str(i))).read_bytes()if(root/('received-'+str(i))).exists()else b''
 def begin_paste(slow=False):
  started.unlink(missing_ok=True)
  if slow:hold.touch()
  else:hold.unlink(missing_ok=True)
  focus('peer');command('peer','set-png',file=image.name);time.sleep(.2);focus('gui');command('gui','focus-view',view=0)
  time.sleep(.1)
  eventually(lambda:(lambda state:state['target_bound']and state['image_offer'])(command('gui','ssh-image-paste')),5)
  command('gui','key',key='v',mods=4)
  if slow:eventually(started.exists,8)
  return command('gui','ssh-image-paste')
 begin_paste();eventually(lambda:len(received(0))>0,12)
 delivered=shlex.split(received(0).decode());assert len(delivered)==1 and digest(Path(delivered[0]))==digest(image) and received(1)==b''
 eventually(lambda:not command('gui','ssh-image-paste')['pending']);assert not a.launch_context or received('decoy')==b'';record('actual-GDK-PNG-Ctrl-V-to-original-PTY-with-space-containing-remote-home')
 baseline=received(0);begin_paste(True);command('gui','tab-select',tab=1);eventually(lambda:not command('gui','ssh-image-paste',view=0)['pending']);hold.unlink(missing_ok=True);time.sleep(3.2)
 assert received(0)==baseline and received(1)==b'';command('gui','tab-select',tab=0)
 record('tab-switch-cancels-upload-with-zero-input-to-either-terminal')
 begin_paste(True);command('gui','key',key='x');eventually(lambda:received(0)==baseline+b'x');hold.unlink(missing_ok=True);eventually(lambda:not command('gui','ssh-image-paste')['pending'],8)
 assert received(0).startswith(baseline+b'x');later=shlex.split(received(0)[len(baseline)+1:].decode());assert len(later)==1 and digest(Path(later[0]))==digest(image)
 record('typing-during-upload-preserved-and-path-delivered-to-same-original-pane')
 extra=rpc('pane.create',{'window':sessions[0]['window'],'orientation':'vertical','argv':[sys.executable,str(receiver),str(root/'received-2')]})
 rpc('pane.wait',{'pane':extra['pane'],'contains':'GENERATED_RECEIVER_READY','timeout_ms':5000})
 command('gui','switch',pane=sessions[0]['pane']);baseline=received(0);begin_paste(True)
 command('gui','switch',pane=extra['pane']);eventually(lambda:not command('gui','ssh-image-paste')['pending']);hold.unlink(missing_ok=True);time.sleep(3.2)
 assert received(0)==baseline and received(2)==b''
 if a.launch_context:
  assert command('gui','ssh-image-paste',paste=True)['handled'] is False
  assert received(2)==b''
  record('local-split-in-same-session-has-no-inherited-SSH-image-destination')
 command('gui','switch',pane=sessions[0]['pane'])
 record('pane-navigation-cancels-image-without-delivering-to-the-new-pane')
 if a.launch_context=='agent':
  command('gui','switch',pane=extra['pane'])
  command('gui','host-shortcuts',operation='header')
  eventually(lambda:any(x['name']=='agent-session'for x in command('gui','host-shortcuts')['widgets']))
  command('gui','host-shortcuts',click='agent-session')
  eventually(lambda:command('gui','inspect')['target']==sessions[0]['pane'])
  assert received(2)==b''
  record('saved-agent-entry-selects-original-pane-rather-than-current-local-split')

 baseline=received(0);begin_paste(True);command('gui','cancel');eventually(lambda:not command('gui','ssh-image-paste')['pending']);hold.unlink(missing_ok=True);time.sleep(3.2);assert received(0)==baseline
 record('clipboard-epoch-change-revokes-pending-image-delivery')
 begin_paste(True);command('gui','ssh-image-paste',reconnect=True);eventually(lambda:not command('gui','ssh-image-paste')['pending']);hold.unlink(missing_ok=True);time.sleep(3.2)
 assert received(0)==baseline and rpc('pane.list',{'session':sessions[0]['session']})[0]['pid']==sessions[0]['pid']
 record('terminal-client-reconnect-cancels-pending-path-with-original-child-preserved')
 begin_paste(True);focus('peer');eventually(lambda:not command('gui','inspect')['active']);eventually(lambda:not command('gui','ssh-image-paste')['pending']);hold.unlink(missing_ok=True);time.sleep(3.2);assert received(0)==baseline;focus('gui');command('gui','focus-view')
 record('native-window-deactivation-revokes-delivery')
 begin_paste(True);command('gui','workspace-close');gui.wait(timeout=10);hold.unlink(missing_ok=True);time.sleep(3.2)
 assert daemon.poll()is None and received(0)==baseline and rpc('pane.list',{'session':sessions[0]['session']})[0]['pid']==sessions[0]['pid']
 record('closing-frontend-cancels-upload-and-preserves-daemon-and-original-pane')
 if a.launch_context:
  # Reopen from disk, then exercise the actual ended-connection button and its
  # rebuilt recipe. The SSH process is a generated fixture child, never an owner session.
  rpc('pane.close',{'pane':extra['pane']})
  (root/'gui-request.json').unlink(missing_ok=True);(root/'gui-response.json').unlink(missing_ok=True)
  gui=start([a.gui,'--new-instance','--socket',sock,'--session',sessions[0]['session'],'--fixture-dir',root],'gui-reconnect-from-context')
  eventually(lambda:command('gui','inspect').get('target')==sessions[0]['pane'],15)
  eventually(lambda:any(e['pane']==sessions[0]['pane']for e in command('gui','launch-contexts')['entries']))
  import signal
  handle=os.pidfd_open(sessions[0]['pid']);signal.pidfd_send_signal(handle,signal.SIGHUP);os.close(handle)
  eventually(lambda:rpc('pane.list',{'session':sessions[0]['session']})[0]['ended'],8);time.sleep(.4)
  previous={e['pane']for e in command('gui','launch-contexts')['entries']}
  button=command('gui','reconnect');assert button['card_visible']and button['ended']and button['clicked'],button
  def new_context():return next((e for e in command('gui','launch-contexts')['entries']if e['pane']not in previous),None)
  replacement=eventually(new_context,12)
  created=rpc('pane.list',{'session':replacement['session']})[0]
  rpc('pane.wait',{'pane':created['id'],'contains':'GENERATED_RECEIVER_READY','timeout_ms':5000})
  assert created['pid']!=sessions[0]['pid']and replacement['provider']==context['provider']
  assert rpc('pane.list',{'session':sessions[0]['session']})[0]['ended']
  record('restored-ended-SSH-reconnect-button-rebuilds-exact-agent-or-terminal-recipe-with-history-preserved')
  # A new explicit launch after engine replacement gets the kernel peer identity
  # of that engine; an old asynchronous catalog result cannot populate it.
  old_entries={e['pane']for e in command('gui','launch-contexts')['entries']}
  old_folder=saved.parent
  engine=Path('/proc')/str(daemon.pid)/'exe';engine=engine.resolve()
  rpc('daemon.stop',{});daemon.wait(timeout=10)
  daemon=start([engine,'--socket',sock,'serve'],'daemon-restarted')
  def ready_again():
   assert daemon.poll()is None
   try:return rpc('health',{})['ok']
   except(OSError,ValueError):return False
  eventually(ready_again,10)
  command('gui','host-shortcuts',click='machine-terminal'if a.launch_context=='ssh'else'machine-codex')
  def after_restart():return next((e for e in command('gui','launch-contexts')['entries']if e['pane']not in old_entries),None)
  latest=eventually(after_restart,15)
  assert not old_entries.intersection(e['pane']for e in command('gui','launch-contexts')['entries'])
  new_record=eventually(lambda:next(iter(catalog.glob('*/'+latest['pane']+'.json')),None))
  assert new_record.parent!=old_folder and saved.exists()
  rpc('pane.wait',{'pane':latest['pane'],'contains':'GENERATED_RECEIVER_READY','timeout_ms':5000})
  record('new-engine-launch-rebinds-catalog-without-replaying-old-daemon-associations')
 (root/'acceptance.json').write_text(json.dumps({'status':'PASS','checks':checks,'scope':'Actual loopback OpenSSH and native GDK/GTK with generated PTYs; synthetic owned Mutter seat, no provider or physical input acceptance'},indent=2)+'\n')
 print('SSH image paste PASS',len(checks),'groups; private evidence',root)
finally:
 try:notify('Stop')
 except Exception:pass
 for child,log in reversed(children):
  if child.poll()is None:child.terminate()
  try:child.wait(timeout=8)
  except subprocess.TimeoutExpired:child.kill();child.wait()
  log.close()
 shutil.rmtree(auth_root)
