#!/usr/bin/env python3
"""Owned GTK startup isolation, real nested tmux input and read-only default import."""
import argparse,json,os,shlex,subprocess,sys,tempfile
from pathlib import Path
from spike import Client,eventually
p=argparse.ArgumentParser();p.add_argument('--binary',type=Path,required=True);p.add_argument('--gui',type=Path,required=True);p.add_argument('--evidence',type=Path,required=True);a=p.parse_args()
if not os.environ.get('WAYLAND_DISPLAY','').startswith('ruwac-'):p.error('Explicit owned test compositor required')
if os.environ.get('RUWAC_NATIVE_OWNED_BUS')!='1':
 result=subprocess.run(['dbus-run-session','--',sys.executable,str(Path(__file__).resolve()),*sys.argv[1:]],env={**os.environ,'RUWAC_NATIVE_OWNED_BUS':'1'},stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True)
 print(result.stdout,end='')
 if result.returncode:print(result.stderr[-2500:],file=sys.stderr)
 raise SystemExit(result.returncode)

a.evidence.mkdir(parents=True,exist_ok=True);root=Path(tempfile.mkdtemp(prefix='native-terminal-usability-',dir=a.evidence));root.chmod(0o700)
env={**os.environ,'GTK_A11Y':'none','GSETTINGS_BACKEND':'memory'}
for key,name in [('HOME','home'),('XDG_CONFIG_HOME','config'),('XDG_DATA_HOME','data'),('XDG_CACHE_HOME','cache')]:env[key]=str(root/name);Path(env[key]).mkdir(mode=0o700)
for key in ['TMUX','STY','RUWAC_SHORTCUTS','RUWAC_KEYMAP']:env.pop(key,None)
home=Path(env['HOME']);data=Path(env['XDG_DATA_HOME']);paths=[]
for i,folder in enumerate([data/'remmina',home/'.remmina',home/'.var/app/org.remmina.Remmina/data/remmina']):
 folder.mkdir(parents=True,mode=0o700);file=folder/(str(i)+'.remmina');file.write_text('[remmina]\nname=Generated Remmina '+str(i)+'\nprotocol=RDP\nserver=127.0.0.1\npassword=GENERATED-DO-NOT-IMPORT\n');paths.append(file)
before={str(path):path.read_bytes() for path in paths};sock=root/'control.sock';children=[];seq=0;fixture=root/'fixture';fixture.mkdir(mode=0o700);checks=[]
def start(argv,name):
 log=(root/(name+'.log')).open('w');child=subprocess.Popen(argv,env=env,stdout=log,stderr=log);children.append((child,log));return child
def rpc(method,params=None):
 with Client(sock)as c:return c.call(method,params)
def command(action,**kw):
 global seq
 seq+=1;tmp=fixture/'request.tmp';tmp.write_text(json.dumps({'seq':seq,'action':action,**kw}));tmp.replace(fixture/'gui-request.json')
 def read():
  try:q=json.loads((fixture/'gui-response.json').read_text());return q['result']if q['seq']==seq else None
  except(OSError,ValueError):return None
 return eventually(read,12)
def gui(*options):
 (fixture/'gui-response.json').unlink(missing_ok=True)
 (fixture/'gui-request.json').unlink(missing_ok=True)
 return start([str(a.gui),'--new-instance','--socket',str(sock),'--fixture-dir',str(fixture),*options],'gui-'+str(len(children)))
def terminate(child):child.terminate();child.wait(timeout=10)
admin=None
try:
 daemon=start([str(a.binary),'--socket',str(sock),'serve'],'daemon');eventually(sock.exists)
 admin=Client(sock);session=admin.call('session.create',{'name':'Generated CLI view','argv':['/bin/cat']})
 admin.call('session.attach',{'session':session['session'],'rows':32,'cols':137})
 first=gui();inspect=eventually(lambda:command('inspect').get('target'),20)
 assert inspect!=session['pane'] and len(rpc('session.list'))==2
 assert (admin.call('client.render')['panes'][0]['grid']['rows'],admin.call('client.render')['panes'][0]['grid']['cols'])==(32,137)
 checks.append('default-GTK-creates-own-session-and-preserves-attached-CLI-size')
 command('profiles-control',import_expanded=True)
 eventually(lambda:command('profiles-control')['discovered_count']==3)
 preview=command('profiles-control',discovered_index=0)
 assert 'Generated Remmina 0'in preview['status'] and preview['import_enabled']
 command('profiles-control',operation='import')
 saved=Path(env['XDG_CONFIG_HOME'])/'RUWAC/machines.json'
 def imported():
  try:return json.loads(saved.read_text())['items']
  except(OSError,ValueError):return None
 profiles=eventually(imported)
 assert len(profiles)==1 and profiles[0].get('secret_reference')==profiles[0]['id']
 assert 'GENERATED-DO-NOT-IMPORT'not in saved.read_text()
 assert all(path.read_bytes()==before[str(path)] for path in paths)
 checks.append('GTK-discovers-native-legacy-flatpak-and-imports-reviewed-metadata-only')
 terminate(first)
 tmux=root/'tmux.sock';conf=root/'tmux.conf';conf.write_text('set -g prefix C-a\nbind C-a send-prefix\nset -g status off\nset -g set-clipboard off\nset -g default-shell /bin/sh\n')
 receiver=Path(__file__).resolve().parents[1]/'fixtures/receiver.py';output=root/'received.bin'
 argv=['tmux','-S',str(tmux),'-f',str(conf),'new-session','-s','generated',shlex.join([sys.executable,str(receiver),str(output)])]
 nested=rpc('session.create',{'name':'Generated nested tmux','argv':argv})
 rpc('pane.wait',{'pane':nested['pane'],'contains':'RUWAC_RECEIVER_READY','timeout_ms':8000})
 second=gui('--session',nested['session']);eventually(lambda:command('inspect').get('target')==nested['pane'],20)
 def key(key,mods=0):command('key',key=key,mods=mods)
 key('a',4);key('a',4);key('c')
 def windows():return subprocess.check_output(['tmux','-S',str(tmux),'list-windows','-t','generated'],env=env,text=True).splitlines()
 eventually(lambda:len(windows())==2)
 assert len(rpc('window.list',{'session':nested['session']}))==1
 key('a',4);key('a',4);key('0');key('x')
 eventually(lambda:output.exists() and output.read_bytes()==b'x')
 checks.append('GTK-double-Control-A-routes-to-real-nested-tmux-without-outer-window-change')
 # The caller runs this suite in its own D-Bus session: a repeated normal GUI
 # invocation activates this fixture's primary application without another PTY.
 primary=start([str(a.gui),'--socket',str(sock)],'primary-application')
 count=len(rpc('session.list'))
 eventually(lambda:len(rpc('session.list'))>count,10)
 count=len(rpc('session.list'))
 repeat=subprocess.run([str(a.gui),'--socket',str(sock)],env=env,capture_output=True,text=True,timeout=10)
 assert repeat.returncode==0 and len(rpc('session.list'))==count and primary.poll()is None
 checks.append('repeated-GTK-launch-reuses-primary-without-an-orphan-session')

 assert daemon.poll()is None and second.poll()is None
 receipt={'status':'PASS','checks':checks,'scope':'Actual GTK on owned Wayland, actual daemon/PTys/tmux and generated Remmina files; GTK callbacks are synthetic, not physical input or external SSH acceptance'}
 (root/'acceptance.json').write_text(json.dumps(receipt,indent=2)+'\n');print('Native terminal usability PASS',len(checks),'groups')
finally:
 if admin:admin.close()
 for child,log in reversed(children):
  if child.poll()is None:child.terminate()
  try:child.wait(timeout=10)
  except subprocess.TimeoutExpired:child.kill();child.wait()
  log.close()
 if (root/'tmux.sock').exists():subprocess.run(['tmux','-S',str(root/'tmux.sock'),'kill-server'],env=env,capture_output=True)
