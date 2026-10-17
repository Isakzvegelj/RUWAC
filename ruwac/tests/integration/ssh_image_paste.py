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
p.add_argument('--evidence',type=Path,required=True);a=p.parse_args()
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
forced=root/'remote-shell';forced.write_text('#!/bin/sh\nexport HOME='+shlex.quote(str(remote))+'\nif test -f '+shlex.quote(str(hold))+'; then touch '+shlex.quote(str(started))+'; sleep 3; fi\nexec /bin/sh -c "$SSH_ORIGINAL_COMMAND"\n');forced.chmod(0o700)
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
 time.sleep(3.2);assert set(target.glob('*.png'))==before;hold.unlink()
 record('cancelled-real-SSH-upload-returns-without-partial-image-publication')

 daemon=start([a.binary,'--socket',sock,'serve'],'daemon');eventually(sock.exists)
 receiver=root/'receiver.py';receiver.write_text('import os,sys,tty\ntty.setraw(0)\nprint("GENERATED_RECEIVER_READY",flush=True)\nwith open(sys.argv[1],"ab",buffering=0)as f:\n while True:\n  b=os.read(0,4096)\n  if not b:break\n  f.write(b)\n')
 sessions=[rpc('session.create',{'name':'Generated '+str(i),'argv':[sys.executable,str(receiver),str(root/('received-'+str(i))) ]})for i in range(2)]
 for s in sessions:rpc('pane.wait',{'pane':s['pane'],'contains':'GENERATED_RECEIVER_READY','timeout_ms':5000})
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
   active=command('gui','inspect')['active'] if who=='gui' else command('peer','inspect')['window_active']
   if active:
    key(65505,True);key(65505,False);time.sleep(.1)
    active=command('gui','inspect')['active'] if who=='gui' else command('peer','inspect')['window_active']
    if active:return
   for n,s in [(65513,True),(65289,True),(65289,False),(65513,False)]:key(n,s)
   time.sleep(.2)
  raise AssertionError('Owned synthetic-seat focus unavailable')
 command('gui','tab-select',tab=0)
 binding=command('gui','ssh-image-paste',profile=profile);assert 'error'not in binding,binding
 received=lambda i:(root/('received-'+str(i))).read_bytes()if(root/('received-'+str(i))).exists()else b''
 def begin_paste(slow=False):
  started.unlink(missing_ok=True)
  if slow:hold.touch()
  else:hold.unlink(missing_ok=True)
  focus('peer');command('peer','set-png',file=image.name);time.sleep(.2);focus('gui');command('gui','focus-view',view=0)
  time.sleep(.1)
  command('gui','key',key='v',mods=4)
  if slow:eventually(started.exists,8)
  return command('gui','ssh-image-paste')
 begin_paste();eventually(lambda:len(received(0))>0,12)
 delivered=shlex.split(received(0).decode());assert len(delivered)==1 and digest(Path(delivered[0]))==digest(image) and received(1)==b''
 eventually(lambda:not command('gui','ssh-image-paste')['pending']);record('actual-GDK-PNG-Ctrl-V-to-original-PTY-with-space-containing-remote-home')
 baseline=received(0);begin_paste(True);command('gui','tab-select',tab=1);eventually(lambda:not command('gui','ssh-image-paste',view=0)['pending']);time.sleep(3.2)
 assert received(0)==baseline and received(1)==b'';command('gui','tab-select',tab=0)
 record('tab-switch-cancels-upload-with-zero-input-to-either-terminal')
 begin_paste(True);command('gui','key',key='x');eventually(lambda:received(0)==baseline+b'x');eventually(lambda:not command('gui','ssh-image-paste')['pending'],8)
 assert received(0).startswith(baseline+b'x');later=shlex.split(received(0)[len(baseline)+1:].decode());assert len(later)==1 and digest(Path(later[0]))==digest(image)
 record('typing-during-upload-preserved-and-path-delivered-to-same-original-pane')
 extra=rpc('pane.create',{'window':sessions[0]['window'],'orientation':'vertical','argv':[sys.executable,str(receiver),str(root/'received-2')]})
 rpc('pane.wait',{'pane':extra['pane'],'contains':'GENERATED_RECEIVER_READY','timeout_ms':5000})
 command('gui','switch',pane=sessions[0]['pane']);baseline=received(0);begin_paste(True)
 command('gui','switch',pane=extra['pane']);eventually(lambda:not command('gui','ssh-image-paste')['pending']);time.sleep(3.2)
 assert received(0)==baseline and received(2)==b'';command('gui','switch',pane=sessions[0]['pane'])
 record('pane-navigation-cancels-image-without-delivering-to-the-new-pane')
 baseline=received(0);begin_paste(True);command('gui','cancel');eventually(lambda:not command('gui','ssh-image-paste')['pending']);time.sleep(3.2);assert received(0)==baseline
 record('clipboard-epoch-change-revokes-pending-image-delivery')
 begin_paste(True);command('gui','ssh-image-paste',reconnect=True);eventually(lambda:not command('gui','ssh-image-paste')['pending']);time.sleep(3.2)
 assert received(0)==baseline and rpc('pane.list',{'session':sessions[0]['session']})[0]['pid']==sessions[0]['pid']
 record('terminal-client-reconnect-cancels-pending-path-with-original-child-preserved')
 begin_paste(True);focus('peer');time.sleep(3.2);assert received(0)==baseline;focus('gui');command('gui','focus-view')
 record('native-window-deactivation-revokes-delivery')
 begin_paste(True);command('gui','workspace-close');gui.wait(timeout=10);time.sleep(3.2)
 assert daemon.poll()is None and received(0)==baseline and rpc('pane.list',{'session':sessions[0]['session']})[0]['pid']==sessions[0]['pid']
 record('closing-frontend-cancels-upload-and-preserves-daemon-and-original-pane')
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
