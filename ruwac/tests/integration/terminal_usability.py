#!/usr/bin/env python3
"""Real named CLI sessions, independent attachments, titles and recursion refusal."""
import argparse,json,os,shlex,subprocess,tempfile,time,fcntl,struct,re
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
 # Assert the actual emitted bottom row, including spaces that make its
 # inverse background visible across the complete terminal width.
 def bar(rows,cols,offset=0):
  pattern=rb'\x1b\['+str(rows).encode()+rb';1H\x1b\[0;38;2;16;32;48;48;2;152;198;232m([^\x1b]*)\x1b\[0m'
  found=re.findall(pattern,one.data[offset:])
  return found[-1].decode() if found and len(found[-1].decode())==cols else None
 assert eventually(lambda:bar(24,80)).startswith('RUWAC d')
 for rows,cols in [(40,120),(120,240),(130,260),(24,80)]:
  offset=len(one.data);fcntl.ioctl(one.slave,0x5414,struct.pack('HHHH',rows,cols,0,0))
  assert eventually(lambda:bar(rows,cols,offset)).startswith('RUWAC d')
 checks.append('full-width-status-bar-at-physical-bottom-through-PTY-limit-resizes')
 checks.append('single-command-start-create-attach-title')
 listed=subprocess.run([str(binary),'--socket',str(sock),'-ls'],env=env,capture_output=True,text=True,timeout=10)
 assert listed.returncode==0 and 'NAME' in listed.stdout and sid in listed.stdout and 'running' in listed.stdout and not listed.stdout.startswith('{')
 machine=subprocess.run([str(binary),'--socket',str(sock),'-list','--json'],env=env,capture_output=True,text=True,timeout=10)
 assert machine.returncode==0 and json.loads(machine.stdout)['result'][0]['name']=='d'
 checks.append('human-readable-and-json-screen-session-list')

 finish(one);assert len(rpc('session.list'))==1
 missing=ui(['-r','absent'],'missing');eventually(lambda:missing.p.poll()is not None);assert missing.p.returncode!=0
 assert len(rpc('session.list'))==1;missing.finish('kill');uis.remove(missing)
 checks.append('reattach-missing-name-never-creates')
 detached=subprocess.run([str(binary),'--socket',str(sock),'-dmS','compat'],env=env,capture_output=True,text=True,timeout=10)
 assert detached.returncode==0 and 'Detached session compat' in detached.stdout
 compat=next(row['id'] for row in rpc('session.list') if row['name']=='compat')
 control=subprocess.run([str(binary),'--socket',str(sock),'-S','compat','-X','title','first'],env=env,capture_output=True,text=True,timeout=10)
 assert control.returncode==0 and rpc('window.list',{'session':compat})[0]['name']=='first'
 view=ui(['-x','compat'],'screen-aliases');eventually(lambda:b'RUWAC compat' in view.data)
 def cmd(text): view.send(b'\x01:'+text.encode()+b'\r');time.sleep(.15)
 cmd('screen');eventually(lambda:len(rpc('window.list',{'session':compat}))==2)
 cmd('title second');eventually(lambda:rpc('window.list',{'session':compat})[1]['name']=='second')
 view.send(b'\x01 ');eventually(lambda:b'0:first' in view.data)
 view.send(b'\x01\x08');eventually(lambda:b'1:second' in view.data)
 cmd('split -v');eventually(lambda:len(rpc('pane.list',{'session':compat}))==3)
 cmd('only');cmd('only')
 view.send(b'\x01k');time.sleep(.15);view.send(b'n');time.sleep(.15);assert len(rpc('window.list',{'session':compat}))==2
 cmd('windowlist');view.send(b'\x1b');time.sleep(.15)
 view.send(b'\x01\x1b');time.sleep(.15);view.send(b'\x1b');time.sleep(.15)
 finish(view)
 for pane in rpc('pane.list',{'session':compat}):rpc('pane.close',{'pane':pane['id']})
 assert len(rpc('session.list'))==1
 checks.append('screen-detached-multiattach-command-and-key-aliases')

 one=ui(['-r','d'],'one');two=ui(['-r','d'],'two');eventually(lambda:b'0:d'in one.data and b'0:d'in two.data)
 assert len(rpc('session.list'))==1 and rpc('session.list')[0]['id']==sid
 one.send(b'\x01c');eventually(lambda:len(rpc('window.list',{'session':sid}))==2)
 eventually(lambda:b'\x1b]2;RUWAC d \xc2\xb7 1:shell'in one.data)
 assert b'\x1b]2;RUWAC d \xc2\xb7 1:shell'not in two.data
 assert two.p.poll()is None
 checks.append('two-real-pty-reattachments-have-independent-window-titles')
 # Actual SGR bottom-row click selects a stable window without reaching a child.
 time.sleep(.15)
 pattern=rb'\x1b\[24;1H\x1b\[0;38;2;16;32;48;48;2;152;198;232m([^\x1b]*)\x1b\[0m'
 line=re.findall(pattern,one.data)[-1].decode()
 column=line.index('0:d')+1
 offset=len(one.data);one.send(f'\x1b[<0;{column};24M\x1b[<0;{column};24m'.encode())
 eventually(lambda:b'\x1b]2;RUWAC d \xc2\xb7 0:d'in one.data[offset:])
 before=len(rpc('pane.list',{'session':sid}))
 one.send(b'\x01s');eventually(lambda:len(rpc('pane.list',{'session':sid}))==before+1)
 one.send(b'\x01|');eventually(lambda:len(rpc('pane.list',{'session':sid}))==before+2)
 checks.append('clickable-physical-bottom-window-bar-and-lowercase-horizontal-vertical-splits')

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
 # Duplicate GUI-style names remain usable by identity, with scoped closure.
 duplicates=[rpc('session.create',{'name':'Terminal','argv':['/bin/bash','--noprofile','--norc']})for _ in range(2)]
 refused=ui(['-r','Terminal'],'ambiguous-name');eventually(lambda:refused.p.poll()is not None);assert refused.p.returncode!=0;refused.finish('kill');uis.remove(refused)
 for index,args in enumerate([['-r',duplicates[0]['session']],['-r','--session',duplicates[1]['session']]]):
  attached=ui(args,'identity-'+str(index));eventually(lambda:b'RUWAC Terminal'in attached.data);finish(attached)
 before={s['id']for s in rpc('session.list')}
 kill=subprocess.run([str(binary),'--socket',str(sock),'kill-session','Terminal'],env=env,capture_output=True,timeout=10);assert kill.returncode!=0
 assert {s['id']for s in rpc('session.list')}==before
 kill=subprocess.run([str(binary),'--socket',str(sock),'kill-session',duplicates[0]['session']],env=env,capture_output=True,timeout=10);assert kill.returncode==0
 assert {s['id']for s in rpc('session.list')}==before-{duplicates[0]['session']}
 control=subprocess.run([str(binary),'--socket',str(sock),'-S',duplicates[1]['session'],'-X','quit'],env=env,capture_output=True,timeout=10);assert control.returncode==0
 assert {s['id']for s in rpc('session.list')}==before-{x['session']for x in duplicates}
 checks.append('duplicate-terminal-name-refusal-exact-ID-reattach-and-scoped-session-closure')
 receipt={'status':'PASS','checks':checks,'scope':'Owned daemon and actual local PTYs; no remote host, physical input or independent PTY-size claim'}
 (root/'acceptance.json').write_text(json.dumps(receipt,indent=2)+'\n');print('Terminal usability PASS',len(checks),'groups')
finally:
 for view in uis:
  try:view.finish('term')
  except Exception:view.p.kill();view.p.wait()
 if sock.exists():
  try:rpc('daemon.stop')
  except Exception:pass
