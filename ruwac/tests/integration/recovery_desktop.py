#!/usr/bin/env python3
"""Real GTK/Wayland reopening and history copy after an owned daemon SIGKILL.
Key commands use the fixture callback; physical keyboard acceptance is separate.
"""
import argparse,hashlib,json,os,signal,subprocess,tempfile,time
from pathlib import Path
from spike import Client,eventually
p=argparse.ArgumentParser();p.add_argument('--binary',required=True);p.add_argument('--gui',required=True);p.add_argument('--evidence',required=True);a=p.parse_args()
root=Path(tempfile.mkdtemp(prefix='recovery-desktop-',dir=a.evidence));root.chmod(0o700)
env={**os.environ,'GTK_A11Y':'none','GSETTINGS_BACKEND':'memory','RUWAC_STATE_FILE':str(root/'durable.json')}
for k,d in [('HOME','home'),('XDG_CONFIG_HOME','config'),('XDG_DATA_HOME','data')]:env[k]=str(root/d);Path(env[k]).mkdir(mode=0o700)
sock=root/'control.sock';seq=0;gui=None;server=None;owned=[];cases=[]
def rpc(method,params=None,error=None):
 with Client(sock)as c:return c.call(method,params,error=error)
def command(action,**kw):
 global seq
 seq+=1;tmp=root/'gui-request.tmp';tmp.write_text(json.dumps({'seq':seq,'action':action,**kw}));tmp.replace(root/'gui-request.json')
 def read():
  try:r=json.loads((root/'gui-response.json').read_text());return r['result']if r['seq']==seq else None
  except (OSError,ValueError):return None
 return eventually(read,10)
def launch(name):
 global gui,seq
 seq=0;gui=subprocess.Popen([a.gui,'--socket',str(sock),'--fixture-dir',str(root)],env=env,stdout=(root/(name+'.log')).open('wb'),stderr=subprocess.STDOUT)
 return eventually(lambda:(r if r.get('panes')==[1]and r.get('drawn',[0])[0]>0 else None)if (r:=command('inspect'))else None,15)
def serve(name):
 global server
 server=subprocess.Popen([a.binary,'--socket',str(sock),'serve'],env=env,stdout=(root/(name+'.log')).open('wb'),stderr=subprocess.STDOUT)
 eventually(lambda:sock.exists()and rpc('health')['ok'])
try:
 serve('before-daemon');one=rpc('session.create',{'name':'Recovered native history','argv':['/bin/bash','--noprofile','--norc']});owned.append(one['pid'])
 rpc('pane.sendText',{'pane':one['pane'],'text':"printf 'NATIVE-RESTORE ž界é🧪\\n'\n"})
 eventually(lambda:'NATIVE-RESTORE ž界é🧪'in rpc('pane.capture',{'pane':one['pane']})['text'])
 launch('before-gui');before=command('mixed-inspect');time.sleep(1)
 checkpoint=env['XDG_CONFIG_HOME']+'/RUWAC/workspace.json';eventually(lambda:Path(checkpoint).exists())
 rpc('daemon.checkpoint');gui.kill();gui.wait(timeout=5);gui=None;server.kill();server.wait(timeout=5);server=None
 serve('after-daemon');restored=launch('after-gui');after=command('mixed-inspect')
 assert before['tabs'][0]['id']==after['tabs'][0]['id']
 assert before['tabs'][0]['layout']['id']==after['tabs'][0]['layout']['id']
 assert 'restored history'in restored['status'].lower(),restored
 # Canonical history enters the existing GTK copy view, with ordinary internal buffer controls.
 command('key',key='a',mods=4);command('key',key='bracketleft');time.sleep(.3)
 command('key',key='g');command('key',key='space');command('key',key='G');command('key',key='Return')
 eventually(lambda:'NATIVE-RESTORE ž界é🧪'in rpc('buffer.get',{'session':one['session']})['text'])
 command('switch',view=0,pane=one['pane']);time.sleep(.4)
 eventually(lambda:((root/'native-rust.png').exists() if command('snapshot') else False),5)
 cases.append({'id':'GTK-DAEMON-CRASH-HISTORY','status':'PASS','display':restored['display'],'same_tab_and_view_ids':True,'visible_ended_status':True,'native_copy_internal_buffer':True,'physical_keys':'not tested; synthetic fixture callbacks'})
 assert command('new-terminal')['clicked'];eventually(lambda:command('inspect')['tabs']==2)
 rows=rpc('session.list');live=next(s for s in rows if s['id']!=one['session']);pid=rpc('pane.list',{'session':live['id']})[0]['pid'];owned.append(pid);assert pid
 rpc('pane.sendText',{'pane':rpc('pane.list',{'session':live['id']})[0]['id'],'text':'printf NEW-LIVE-SESSION\n'})
 eventually(lambda:'NEW-LIVE-SESSION'in rpc('pane.capture',{'pane':rpc('pane.list',{'session':live['id']})[0]['id']})['text'])
 cases.append({'id':'GTK-RECOVERY-NEW-TERMINAL','status':'PASS','new_live_session_alongside_read_only_history':True})
 command('workspace-close');gui.wait(timeout=5);gui=None;rpc('daemon.stop');server.wait(timeout=5);server=None
finally:
 for proc in [gui,server]:
  if proc and proc.poll()is None:proc.kill();proc.wait(timeout=5)
 for pid in owned:
  try:
   if os.getsid(pid)==pid:os.killpg(pid,signal.SIGKILL)
  except ProcessLookupError:pass
 receipt={'cases':cases,'hashes':{n:hashlib.sha256(Path(b).read_bytes()).hexdigest()for n,b in [('ruwac',a.binary),('ruwac-desktop',a.gui)]},'evidence':str(root)}
 (root/'receipt.json').write_text(json.dumps(receipt,indent=2)+'\n');print(json.dumps(receipt,indent=2))
