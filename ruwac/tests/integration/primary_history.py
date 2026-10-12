#!/usr/bin/env python3
"""Actual PTY dual-buffer checkpoint/recovery and optional owned native copy controls."""
import argparse,hashlib,json,os,signal,subprocess,sys,tempfile,time
from pathlib import Path
from spike import Client,eventually
p=argparse.ArgumentParser();p.add_argument('--binary',required=True);p.add_argument('--gui');p.add_argument('--evidence',type=Path,required=True);a=p.parse_args()
if a.gui and (os.environ.get('GDK_BACKEND')!='wayland' or not Path(os.environ.get('XDG_RUNTIME_DIR','/')).resolve().is_relative_to(a.evidence.resolve())):raise SystemExit('Native primary history requires an owned Wayland runtime inside evidence')
root=Path(tempfile.mkdtemp(prefix='primary-history-',dir=a.evidence));root.chmod(0o700);sock=root/'control.sock';state=root/'durable.json';env={**os.environ,'RUWAC_STATE_FILE':str(state),'GTK_A11Y':'none','GSETTINGS_BACKEND':'memory'}
for key,name in [('HOME','home'),('XDG_CONFIG_HOME','config'),('XDG_DATA_HOME','data'),('XDG_CACHE_HOME','cache')]:env[key]=str(root/name);Path(env[key]).mkdir(mode=0o700)
app=root/'app.py';app.write_text('''import os,tty
# Canonical output comes from a real owned PTY process, not injected screen JSON.
for i in range(1000):os.write(1,(f"PRIMARY-{i:04} Živjo 界e\\u0301\\r\\n").encode())
os.write(1,b"\\x1b[?1049h\\x1b[38;2;1;2;3mALT-APPLICATION-READY")
tty.setraw(0)
while True:
 data=os.read(0,1)
 if not data:break
 if data==b"a":os.write(1,b"-ALT-UPDATE")
 if data==b"x":os.write(1,b"\\x1b[?1049lPRIMARY-RETURNED\\r\\n")
''');server=None;gui=None;logs=[];results=[];seq=0;owned_pid=None
def start(name='daemon',path=state):
 global server
 conf={**env,'RUWAC_STATE_FILE':str(path)};f=(root/(name+'.log')).open('wb');logs.append(f);server=subprocess.Popen([a.binary,'--socket',str(sock),'serve'],env=conf,stdout=f,stderr=f)
 eventually(lambda:health(),10)
def health():
 try:
  with Client(sock)as c:return c.call('health')
 except (OSError,EOFError):return None
def rpc(method,q=None,error=None):
 with Client(sock)as c:return c.call(method,q,error=error)
def page(buffer='active'):
 out=[];offset=0;epoch=None
 while True:
  q={'pane':pane,'offset':offset,'lines':500,'buffer':buffer}
  if epoch is not None:q['epoch']=epoch
  r=rpc('pane.history',q);epoch=r['epoch'];out.append(r['text'])
  if r['next_offset']>=r['total_lines']:break
  assert r['next_offset']>offset,r;offset=r['next_offset']
 return ''.join(out)
def command(action,**kw):
 global seq
 seq+=1;temp=root/'request.tmp';temp.write_text(json.dumps({'seq':seq,'action':action,**kw}));temp.replace(root/'gui-request.json')
 def read():
  try:d=json.loads((root/'gui-response.json').read_text());return d['result']if d['seq']==seq else None
  except(OSError,ValueError):return None
 return eventually(read,10)
def record(name,**details):results.append({'id':name,'status':'PASS',**details});print(name,'PASS',flush=True)
def stop():
 global server
 rpc('daemon.stop');server.wait(timeout=10);server=None
try:
 start();session=rpc('session.create',{'name':'Dual canonical buffers','argv':[sys.executable,str(app)]});pane=session['pane'];owned_pid=session['pid']
 eventually(lambda:'ALT-APPLICATION-READY'in rpc('pane.capture',{'pane':pane})['text'],15)
 active=page();primary=page('primary');assert 'ALT-APPLICATION-READY'in active and 'PRIMARY-0999 Živjo 界e\u0301'in primary and 'PRIMARY-0999'not in active
 before=rpc('pane.capture',{'pane':pane})['text'];page('primary');assert rpc('pane.capture',{'pane':pane})['text']==before
 cli=json.loads(subprocess.check_output([a.binary,'--socket',str(sock),'pane','capture','--id',pane,'--lines','2000','--buffer','primary','--json'],env=env,text=True));assert cli['result']['buffer']=='primary'and'PRIMARY-0999'in cli['result']['text']
 record('PRIMARY-REAL-PTY-SEPARATION',unicode_combining=True,CLI_selector=True,active_unchanged=True)
 with Client(sock)as c:
  frozen=c.call('pane.history.freeze',{'pane':pane,'buffer':'primary'});snapshot=frozen['snapshot'];old=c.call('pane.history',{'pane':pane,'snapshot':snapshot,'buffer':'primary','offset':0,'lines':500})
  c.call('pane.sendText',{'pane':pane,'text':'a'});eventually(lambda:'ALT-UPDATE'in rpc('pane.capture',{'pane':pane})['text'])
  assert c.call('pane.history',{'pane':pane,'snapshot':snapshot,'buffer':'primary','offset':0,'lines':500})==old
  c.call('pane.history',{'pane':pane,'snapshot':snapshot,'buffer':'active'},error='INVALID_PARAMS');c.call('pane.history.release',{'snapshot':snapshot})
 rpc('pane.history',{'pane':pane,'buffer':'invalid'},error='INVALID_PARAMS')
 record('PRIMARY-IMMUTABLE-SNAPSHOT',buffer_mismatch_refused=True,live_ALT_update_preserved=True)
 if a.gui:
  f=(root/'gui.log').open('wb');logs.append(f);gui=subprocess.Popen([a.gui,'--socket',str(sock),'--session',session['session'],'--fixture-dir',str(root)],env=env,stdout=f,stderr=f);eventually(lambda:command('inspect'),15)
  assert command('history-buffer',buffer='primary')['clicked'];expected=hashlib.sha256(page('primary').encode()).hexdigest();eventually(lambda:r if(r:=command('copy-state'))['text_sha256']==expected else None,15)
  assert command('history-buffer',buffer='active')['clicked'];expected=hashlib.sha256(page().encode()).hexdigest();eventually(lambda:r if(r:=command('copy-state'))['text_sha256']==expected else None,15)
  command('workspace-close');gui.wait(timeout=10);gui=None;record('PRIMARY-NATIVE-BUFFER-BUTTONS',actual_GTK_callbacks=True,no_clipboard_offer=True,physical_input=False)
 active=page();primary=page('primary');checkpoint=rpc('daemon.checkpoint');saved=state.read_bytes();d=json.loads(saved);assert d['schema']==3 and d['panes'][0]['terminal']['primary']['grid']['alternate_screen'] is False
 server.kill();server.wait(timeout=10);server=None
 start('restored');assert page()==active and page('primary')==primary
 rows=rpc('pane.list',{'session':session['session']});assert rows[0]['ended']and rows[0]['restored']and rows[0]['pid']is None
 rpc('pane.sendText',{'pane':pane,'text':'not delivered'},error='PANE_EXITED');record('PRIMARY-ACTUAL-SIGKILL-RECOVERY',both_buffers_exact=True,ended_without_resurrection=True)
 stop();legacy=json.loads(saved);legacy['schema']=2;legacy['panes'][0]['terminal'].pop('primary');old=root/'legacy.json';old.write_text(json.dumps(legacy));old.chmod(0o600);start('legacy',old)
 assert page()==active;rpc('pane.history',{'pane':pane,'buffer':'primary'},error='BUFFER_UNAVAILABLE');stop();record('PRIMARY-LEGACY-ALT-EXPLICIT-UNAVAILABLE',active_history_preserved=True)
 bad=json.loads(saved);bad['panes'][0]['terminal']['primary']['history'].append(['\u001b[2J',False]);path=root/'bad-primary.json';path.write_text(json.dumps(bad));path.chmod(0o600);before=path.read_bytes()
 f=(root/'rejected.log').open('wb');logs.append(f);proc=subprocess.Popen([a.binary,'--socket',str(root/'rejected.sock'),'serve'],env={**env,'RUWAC_STATE_FILE':str(path)},stdout=f,stderr=f);assert proc.wait(timeout=10)!=0 and path.read_bytes()==before
 record('PRIMARY-CORRUPT-CHECKPOINT-REJECTED',original_preserved=True,no_escape_replay=True)
finally:
 if gui is not None and gui.poll()is None:gui.terminate();gui.wait(timeout=10)
 if server is not None and server.poll()is None:
  try:stop()
  except Exception:server.kill();server.wait(timeout=5)
 if owned_pid:
  try:
   cmd=Path(f'/proc/{owned_pid}/cmdline').read_bytes()
   if str(app).encode()in cmd:os.kill(owned_pid,signal.SIGTERM)
  except FileNotFoundError:pass
 for f in logs:f.close()
 receipt={'cases':results,'sha256':{'ruwac':hashlib.sha256(Path(a.binary).read_bytes()).hexdigest(),**({'ruwac-desktop':hashlib.sha256(Path(a.gui).read_bytes()).hexdigest()}if a.gui else{})},'scope':'Real owned PTY/daemon SIGKILL and canonical buffers; native actions on owned display only; no owner sessions/configuration/clipboard','evidence':str(root)}
 (root/'receipt.json').write_text(json.dumps(receipt,indent=2)+'\n');(a.evidence/'primary-history-acceptance.json').write_text(json.dumps(receipt,indent=2)+'\n');print('Evidence',root,flush=True)
assert len(results)==(6 if a.gui else 5)
