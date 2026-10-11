#!/usr/bin/env python3
"""Actual older daemon/new GTK client compatibility on an owned compositor."""
import argparse,hashlib,json,os,signal,subprocess,sys,tempfile,time
from pathlib import Path
from spike import Client,eventually
p=argparse.ArgumentParser();p.add_argument('--old-binary',required=True);p.add_argument('--gui',required=True);p.add_argument('--evidence',required=True);a=p.parse_args()
root=Path(tempfile.mkdtemp(prefix='previous-compat-',dir=a.evidence));root.chmod(0o700);sock=root/'control.sock'
env={**os.environ,'HOME':str(root/'home'),'XDG_CONFIG_HOME':str(root/'config'),'XDG_DATA_HOME':str(root/'data'),'XDG_CACHE_HOME':str(root/'cache'),'GSETTINGS_BACKEND':'memory','GTK_A11Y':'none','GTK_USE_PORTAL':'0'}
for key in ['HOME','XDG_CONFIG_HOME','XDG_DATA_HOME','XDG_CACHE_HOME']:Path(env[key]).mkdir(mode=0o700)
server=None;gui=None;child=None;seq=0;results=[]
def rpc(method,params=None):
 with Client(sock)as c:return c.call(method,params)
def command(action,**kwargs):
 global seq
 seq+=1;request=root/'gui-request.tmp';request.write_text(json.dumps({'seq':seq,'action':action,**kwargs}));request.replace(root/'gui-request.json')
 def read():
  try:r=json.loads((root/'gui-response.json').read_text());return r['result']if r['seq']==seq else None
  except (FileNotFoundError,ValueError):return None
 return eventually(read,8)
try:
 server=subprocess.Popen([a.old_binary,'--socket',str(sock),'serve'],env=env,stdout=(root/'daemon.log').open('wb'),stderr=subprocess.STDOUT)
 eventually(lambda:sock.exists()and rpc('health')['ok'])
 assert 'window.previous' not in rpc('schema')['methods'],'This test requires the preceding daemon API'
 receiver=Path(__file__).resolve().parents[1]/'fixtures/receiver.py';output=root/'input.bin'
 session=rpc('session.create',{'name':'Older daemon compatibility','argv':[sys.executable,str(receiver),str(output)]});child=session['pid']
 rpc('pane.wait',{'pane':session['pane'],'contains':'RUWAC_RECEIVER_READY','timeout_ms':5000})
 gui=subprocess.Popen([a.gui,'--socket',str(sock),'--session',session['session'],'--fixture-dir',str(root),'--clipboard-disabled'],env=env,stdout=(root/'gui.log').open('wb'),stderr=subprocess.STDOUT)
 eventually(lambda:command('inspect')['target']==session['pane'])
 command('key',key='a',mods=4);command('key',key='a',mods=4)
 eventually(lambda:'requires a newer daemon'in command('inspect')['status'])
 command('switch',pane=session['pane'],text='AFTER-UNSUPPORTED-PREVIOUS')
 eventually(lambda:output.read_bytes()==b'AFTER-UNSUPPORTED-PREVIOUS')
 assert rpc('pane.list',{'session':session['session']})[0]['pid']==child
 results.append({'id':'GTK-OLDER-DAEMON-PREVIOUS','status':'PASS','unsupported_capability_visible':True,'input_not_frozen':True,'same_child_pid':True,'events':'real GTK client callbacks; synthetic keys','clipboard_disabled':True})
finally:
 if gui and gui.poll()is None:gui.terminate();gui.wait(timeout=5)
 if server and server.poll()is None:
  try:rpc('daemon.stop');server.wait(timeout=5)
  except Exception:server.kill();server.wait(timeout=5)
 if child:
  try:
   if os.getsid(child)==child:os.killpg(child,signal.SIGKILL)
  except ProcessLookupError:pass
 receipt={'results':results,'old_daemon_sha256':hashlib.sha256(Path(a.old_binary).read_bytes()).hexdigest(),'gui_sha256':hashlib.sha256(Path(a.gui).read_bytes()).hexdigest(),'evidence':str(root)}
 (root/'receipt.json').write_text(json.dumps(receipt,indent=2)+'\n');print(json.dumps(receipt,indent=2))
assert len(results)==1
