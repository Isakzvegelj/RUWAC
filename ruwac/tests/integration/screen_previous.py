#!/usr/bin/env python3
"""Screen previous/literal prefix through real PTYs; isolated checkpoint migration."""
import argparse,hashlib,json,os,signal,subprocess,sys,tempfile,time
from pathlib import Path
from spike import Client,Tui,eventually
p=argparse.ArgumentParser();p.add_argument('--binary',required=True);p.add_argument('--evidence',required=True);a=p.parse_args()
binary=Path(a.binary).resolve();root=Path(tempfile.mkdtemp(prefix='screen-previous-',dir=a.evidence));root.chmod(0o700)
sock=root/'control.sock';archive=root/'history.json';env={**os.environ,'RUWAC_STATE_FILE':str(archive)};server=None;tui=None;owned=[];results=[]
def rpc(method,params=None,error=None):
 with Client(sock) as c:return c.call(method,params,error=error)
def start():
 global server
 server=subprocess.Popen([str(binary),'--socket',str(sock),'serve'],env=env,stdout=(root/'daemon.log').open('ab'),stderr=subprocess.STDOUT)
 eventually(lambda:sock.exists() and rpc('health')['ok'])
def stop():
 global server
 rpc('daemon.stop');assert server.wait(timeout=10)==0;server=None
try:
 start();receiver=Path(__file__).resolve().parents[1]/'fixtures/receiver.py'
 first=rpc('session.create',{'name':'Screen previous','argv':[sys.executable,str(receiver),str(root/'input.bin')]});owned.append(first['pid']);session=first['session']
 rpc('pane.wait',{'pane':first['pane'],'contains':'RUWAC_RECEIVER_READY','timeout_ms':5000})
 second=rpc('window.create',{'session':session});third=rpc('window.create',{'session':session})
 owned.extend(x['pid'] for x in rpc('pane.list',{'session':session})if x['pid'] not in owned)
 tui=Tui(binary,sock,session,root/'tui.bin');eventually(lambda:b'RUWAC' in tui.data)
 tui.send(b'\x01\x01');eventually(lambda:rpc('session.list')[0]['active_window']==second['window'])
 tui.send(b'\x01\x01');eventually(lambda:rpc('session.list')[0]['active_window']==third['window'])
 tui.send(b'\x010');eventually(lambda:rpc('session.list')[0]['active_window']==first['window'])
 tui.send(b'\x01a');eventually(lambda:(root/'input.bin').exists() and (root/'input.bin').read_bytes()==b'\x01')
 assert rpc('session.list')[0]['active_window']==first['window']
 tui.finish();tui=None
 assert next(x for x in rpc('pane.list',{'session':session})if x['id']==first['pane'])['pid']==first['pid']
 results.append({'id':'SCREEN-PREVIOUS-LITERAL-PTY','status':'PASS','double_prefix_previous':True,'literal_prefix_exact':True,'same_child_pid':True,'termios_restored':True})
 rpc('daemon.checkpoint');before=rpc('session.list')[0];stop();start();after=rpc('session.list')[0]
 assert before['previous_window']==after['previous_window']==third['window']
 assert rpc('window.previous',{'session':session})['selected']==third['window']
 rpc('pane.close',{'pane':first['pane']});assert rpc('session.list')[0]['previous_window'] is None
 stop();saved=json.loads(archive.read_text());assert saved['schema']==5
 saved['schema']=1
 for item in saved['sessions']:item.pop('previous',None)
 archive.write_text(json.dumps(saved));archive.chmod(0o600);start()
 assert rpc('session.list')[0]['previous_window'] is None
 expected=second['window'];assert rpc('window.previous',{'session':session})['selected']==expected
 rpc('daemon.checkpoint');assert json.loads(archive.read_text())['schema']==5
 results.append({'id':'SCREEN-PREVIOUS-RECOVERY','status':'PASS','previous_persisted':True,'closed_previous_removed':True,'schema1_migration':True,'missing_previous_cycles_next':True})
finally:
 if tui:tui.finish('term')
 if server and server.poll() is None:server.kill();server.wait(timeout=5)
 for pid in owned:
  try:
   if os.getsid(pid)==pid:os.killpg(pid,signal.SIGKILL)
  except ProcessLookupError:pass
 receipt={'results':results,'binary_sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),'evidence':str(root)}
 (root/'receipt.json').write_text(json.dumps(receipt,indent=2)+'\n');print(json.dumps(receipt,indent=2))
assert len(results)==2
