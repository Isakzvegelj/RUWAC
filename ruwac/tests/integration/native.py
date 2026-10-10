#!/usr/bin/env python3
"""Real native Wayland paint/client-lifecycle test with synthetic, isolated PTYs."""
import argparse, importlib.util, json, os, subprocess, sys, tempfile, time
from pathlib import Path
parser=argparse.ArgumentParser();parser.add_argument('--binary',required=True);parser.add_argument('--evidence',required=True);parser.add_argument('--gui',required=True);a=parser.parse_args()
spec=importlib.util.spec_from_file_location('spike',Path(__file__).with_name('spike.py'));spike=importlib.util.module_from_spec(spec);spec.loader.exec_module(spike)
root=Path(tempfile.mkdtemp(prefix='native-',dir=a.evidence));root.chmod(0o700);sock=root/'control.sock';binary=Path(a.binary).resolve();receiver=Path(__file__).parents[1]/'fixtures/receiver.py';log=(root/'daemon.log').open('wb');server=subprocess.Popen([str(binary),'--socket',str(sock),'serve'],stdout=log,stderr=log);gui=None
try:
 spike.eventually(lambda:sock.exists())
 def call(m,p=None):
  with spike.Client(sock) as client:return client.call(m,p)
 def create(name,file):return call('session.create',{'name':name,'argv':[sys.executable,str(receiver),str(file)]})
 s=create('Native fixture one',root/'one.bin');other=create('Native fixture two',root/'two.bin')
 split=call('pane.create',{'window':s['window'],'argv':[sys.executable,str(receiver),str(root/'split.bin')]})
 panes=[s['pane'],split['pane'],other['pane']];pids=[s['pid'],split['pid'],other['pid']]
 for pane in panes:call('pane.wait',{'pane':pane,'contains':'READY','timeout_ms':5000})
 payload='literal $HOME "quotes" \\ žółć 🧪\t\n'
 for pane in panes:call('pane.sendText',{'pane':pane,'text':payload})
 for file in (root/'one.bin',root/'two.bin',root/'split.bin'):spike.eventually(lambda f=file:f.read_bytes()==payload.encode())
 report=root/'native-gui.json'
 with (root/'gui.log').open('wb') as output:
  gui=subprocess.Popen([sys.executable,a.gui,'--socket',str(sock),'--session',s['session'],'--session',other['session'],'--self-test-report',str(report)],stdout=output,stderr=output)
  gui.wait(timeout=30)
 assert gui.returncode==0,(root/'gui.log').read_text()
 result=json.loads(report.read_text());assert result['status']=='PASS',result
 assert result['display']=='GdkWaylandDisplay' and result['tabs']==2 and result['panes']==[2,1],result
 assert result['image_width']>100 and report.with_suffix('.png').stat().st_size>1000
 assert all('READY' in text for tab in result['rendered_text'] for text in tab),result
 spike.eventually(lambda:(root/'one.bin').read_bytes()==payload.encode()+b'GUI!\x01')
 assert (root/'split.bin').read_bytes()==payload.encode()
 assert (root/'two.bin').read_bytes()==payload.encode()
 for pid in pids:os.kill(pid,0)
 # Reattach using an independent native protocol adapter after GUI exit.
 sys.path.insert(0,str(Path(a.gui).parent));from bridge import Rpc
 rpc=Rpc(sock)
 try:
  rpc.call('session.attach',{'session':s['session'],'rows':30,'cols':90})
  frame=rpc.call('client.render');assert {p['id'] for p in frame['panes']}=={s['pane'],split['pane']}
  rpc.call('client.input',{'pane':s['pane'],'kind':'keyboard','text':'AFTER_GUI'})
  spike.eventually(lambda:(root/'one.bin').read_bytes()==payload.encode()+b'GUI!\x01AFTER_GUI')
  rpc.call('client.resize',{'rows':34,'cols':94});frame=rpc.call('client.render');assert all(p['grid']['cols']==94 for p in frame['panes'])
  rpc.call('session.detach')
 finally:rpc.close()
 result.update({'gui_exit_keeps_pids':pids,'reattach_and_targeted_input':'PASS','resize':'PASS','socket_parent_mode':oct(root.stat().st_mode&0o777),'socket_mode':oct(sock.stat().st_mode&0o777),'binary_sha256':__import__('hashlib').sha256(binary.read_bytes()).hexdigest()})
 report.write_text(json.dumps(result,indent=2)+'\n');print('NATIVE_ACCEPTANCE',root)
finally:
 if gui and gui.poll() is None:gui.terminate();gui.wait(timeout=5)
 if server.poll() is None:
  try:call('daemon.stop')
  except Exception:server.terminate()
  server.wait(timeout=8)
 log.close()
