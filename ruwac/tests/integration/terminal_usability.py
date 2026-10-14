#!/usr/bin/env python3
"""Real named CLI sessions, independent attachments, titles and recursion refusal."""
import argparse,json,os,shlex,subprocess,tempfile,time
from pathlib import Path
from spike import Client,Tui,eventually
p=argparse.ArgumentParser();p.add_argument('--binary',type=Path,required=True);p.add_argument('--evidence',type=Path,required=True);a=p.parse_args()
a.evidence.mkdir(parents=True,exist_ok=True);root=Path(tempfile.mkdtemp(prefix='terminal-usability-',dir=a.evidence));root.chmod(0o700)
binary=a.binary.resolve();sock=root/'control.sock';env={**os.environ,'TERM':'xterm-256color','RUWAC_STATE_FILE':str(root/'history.json')}
for key,name in [('HOME','home'),('XDG_CONFIG_HOME','config'),('XDG_DATA_HOME','data'),('XDG_CACHE_HOME','cache')]:env[key]=str(root/name);Path(env[key]).mkdir(mode=0o700)
uis=[];checks=[]
def rpc(method,params=None):
 with Client(sock)as c:return c.call(method,params)
def ui(command,name,cols=80):
 view=Tui(binary,sock,'',root/(name+'.pty'),env=env,command=command,cols=cols);uis.append(view);return view
def finish(view):
 assert view.finish()==0;uis.remove(view)
try:
 refusal=subprocess.run([str(binary),'--socket',str(sock),'-S','d'],env=env,capture_output=True,text=True,timeout=10)
 assert refusal.returncode!=0 and not sock.exists()
 one=ui(['-S','d'],'create');eventually(lambda:sock.exists())
 sessions=eventually(lambda:rpc('session.list'));assert len(sessions)==1 and sessions[0]['name']=='d';sid=sessions[0]['id']
 eventually(lambda:b'RUWAC d'in one.data);assert b'\x1b]2;RUWAC d'in one.data
 checks.append('single-command-start-create-attach-title')
 finish(one);assert len(rpc('session.list'))==1
 missing=ui(['-r','absent'],'missing');eventually(lambda:missing.p.poll()is not None);assert missing.p.returncode!=0
 assert len(rpc('session.list'))==1;missing.finish('kill');uis.remove(missing)
 checks.append('reattach-missing-name-never-creates')
 one=ui(['-r','d'],'one');two=ui(['-r','d'],'two');eventually(lambda:b'0:d'in one.data and b'0:d'in two.data)
 assert len(rpc('session.list'))==1 and rpc('session.list')[0]['id']==sid
 one.send(b'\x01c');eventually(lambda:len(rpc('window.list',{'session':sid}))==2)
 eventually(lambda:b'\x1b]2;RUWAC d \xc2\xb7 1:shell'in one.data)
 assert b'\x1b]2;RUWAC d \xc2\xb7 1:shell'not in two.data
 assert two.p.poll()is None
 checks.append('two-real-pty-reattachments-have-independent-window-titles')
 one.send(b'\x010');time.sleep(.15)
 session=rpc('pane.list',{'session':sid})[0]
 command=shlex.join([str(binary),'--socket',str(sock),'-r','d'])+'\n'
 rpc('pane.sendText',{'pane':session['id'],'text':command})
 eventually(lambda:'Cannot attach a session from inside its own pane'in rpc('pane.capture',{'pane':session['id']})['text'])
 assert one.p.poll()is None and two.p.poll()is None
 checks.append('recursive-self-attachment-refused-before-raw-mode')
 finish(one);finish(two)
 assert b'\x1b[22;0t'in one.data and b'\x1b[23;0t'in one.data
 checks.append('screen-detach-preserves-session-and-restores-title-stack')
 receipt={'status':'PASS','checks':checks,'scope':'Owned daemon and actual local PTYs; no remote host, physical input or independent PTY-size claim'}
 (root/'acceptance.json').write_text(json.dumps(receipt,indent=2)+'\n');print('Terminal usability PASS',len(checks),'groups')
finally:
 for view in uis:
  try:view.finish('term')
  except Exception:view.p.kill();view.p.wait()
 if sock.exists():
  try:rpc('daemon.stop')
  except Exception:pass
