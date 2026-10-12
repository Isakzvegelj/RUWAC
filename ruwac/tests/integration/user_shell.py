#!/usr/bin/env python3
"""User shell/login startup and GUI compatibility with an existing older daemon."""
import argparse,hashlib,json,os,subprocess,tempfile,time
from pathlib import Path
from spike import Client,eventually
p=argparse.ArgumentParser();p.add_argument('--binary',required=True);p.add_argument('--gui',required=True);p.add_argument('--legacy',required=True);p.add_argument('--evidence',type=Path,required=True);a=p.parse_args()
runtime=Path(os.environ['XDG_RUNTIME_DIR']);assert runtime.name=='display'and runtime.parent.parent.name=='results'
root=Path(tempfile.mkdtemp(prefix='user-shell-',dir=a.evidence));root.chmod(0o700);env={**os.environ,'GTK_A11Y':'none','GSETTINGS_BACKEND':'memory'}
for key,name in [('HOME','home'),('XDG_CONFIG_HOME','config'),('XDG_DATA_HOME','data'),('XDG_CACHE_HOME','cache')]:env[key]=str(root/name);Path(env[key]).mkdir(mode=0o700)
# Generated shell sources only a fixture-owned profile. No owner shell files are read.
profile=Path(env['HOME'])/'.bash_profile';profile.write_text("printf 'RUWAC_USER_LOGIN_PROFILE_LOADED\\n'\n")
shell=root/'selected-user-shell';shell.write_text('#!/bin/sh\n[ "$1" = "-l" ] || exit 13\n. "$HOME/.bash_profile"\nexec /bin/cat\n');shell.chmod(0o700);env['SHELL']=str(shell);children=[];seq=0;results=[]
def start(argv,name):
 log=(root/(name+'.log')).open('wb');process=subprocess.Popen(argv,env=env,stdout=log,stderr=log);children.append((process,log));return process

def command(action):
 global seq
 seq+=1;tmp=root/'request.tmp';tmp.write_text(json.dumps({'seq':seq,'action':action}));tmp.replace(root/'gui-request.json')
 def response():
  try:d=json.loads((root/'gui-response.json').read_text());return d['result']if d['seq']==seq else None
  except (OSError,ValueError):return None
 return eventually(response,10)
try:
 sock=root/'modern.sock';start([a.binary,'--socket',str(sock),'serve'],'modern');eventually(lambda:sock.exists())
 with Client(sock)as c:
  session=c.call('session.create',{'name':'Default user shell'})
  eventually(lambda:'RUWAC_USER_LOGIN_PROFILE_LOADED'in c.call('pane.capture',{'pane':session['pane']})['text'])
 results.append({'id':'USER-SHELL-DAEMON-DEFAULT-LOGIN-PROFILE','status':'PASS'})
 legacy=root/'legacy.sock';old=start([a.legacy,'--socket',str(legacy),'serve'],'legacy');eventually(lambda:legacy.exists())
 with Client(legacy)as c:initial=c.call('session.create',{'name':'Existing older daemon session','argv':['/bin/cat']})
 gui=start([a.gui,'--socket',str(legacy),'--session',initial['session'],'--fixture-dir',str(root)],'gui');eventually(lambda:command('inspect'),15);assert command('new-terminal')['clicked']
 with Client(legacy)as c:
  def created():
   rows=c.call('session.list');return next((r for r in rows if r['id']!=initial['session']),None)
  new=eventually(created)
  panes=c.call('pane.list',{'session':new['id']});pane=panes[0]['id']
  eventually(lambda:'RUWAC_USER_LOGIN_PROFILE_LOADED'in c.call('pane.capture',{'pane':pane})['text'])
  c.call('pane.sendText',{'pane':initial['pane'],'text':'EXISTING-SESSION-PRESERVED\n'})
  eventually(lambda:'EXISTING-SESSION-PRESERVED'in c.call('pane.capture',{'pane':initial['pane']})['text'])
 assert old.poll()is None
 results.append({'id':'USER-SHELL-GTK-NEW-TERMINAL-OLDER-DAEMON-COMPATIBILITY','status':'PASS'})
 command('window-close');eventually(lambda:gui.poll()is not None)
 assert old.poll()is None
 results.append({'id':'USER-SHELL-CLOSE-WINDOW-PRESERVES-DAEMON-PTY','status':'PASS'})
finally:
 for process,log in reversed(children):
  if process.poll()is None:
   process.terminate()
   try:process.wait(timeout=8)
   except subprocess.TimeoutExpired:process.kill();process.wait()
  log.close()
print(json.dumps({'results':results,'sha256':{n:hashlib.sha256(Path(v).read_bytes()).hexdigest()for n,v in [('ruwac',a.binary),('ruwac-desktop',a.gui)]},'scope':'Generated shell/profile in private HOME, actual modern/older daemons and native GTK callbacks'},indent=2))
