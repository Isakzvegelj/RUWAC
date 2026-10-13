#!/usr/bin/env python3
"""Public named CLI contract, file input and explicit-target negatives."""
import argparse, importlib.util, json, subprocess, sys, tempfile
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('--binary',required=True);p.add_argument('--evidence',required=True);a=p.parse_args()
spec=importlib.util.spec_from_file_location('spike',Path(__file__).with_name('spike.py'));spike=importlib.util.module_from_spec(spec);spec.loader.exec_module(spike)
root=Path(tempfile.mkdtemp(prefix='named-cli-',dir=a.evidence));root.chmod(0o700);sock=root/'control.sock';binary=str(Path(a.binary).resolve());receiver=Path(__file__).parents[1]/'fixtures/receiver.py';log=(root/'daemon.log').open('wb');server=subprocess.Popen([binary,'--socket',str(sock),'serve'],stdout=log,stderr=log)
def run(*args,ok=True):
 result=subprocess.run([binary,'--socket',str(sock),*args],capture_output=True,timeout=10)
 assert (result.returncode==0)==ok,(args,result.stdout,result.stderr)
 return json.loads(result.stdout) if result.stdout else None
try:
 spike.eventually(lambda:sock.exists())
 with spike.Client(sock) as rpc:
  s=rpc.call('session.create',{'name':'named-fixture','argv':[sys.executable,str(receiver),str(root/'input.bin')]})
  rpc.call('pane.wait',{'pane':s['pane'],'contains':'READY','timeout_ms':5000})
 revision=run('health')['result']['revision'];description=run('describe')['result']
 assert 'describe'in description['schema']['methods'] and description['protocol']==0
 assert run('health')['result']['revision']==revision
 assert run('session','list','--json')['result'][0]['id']==s['session']
 assert run('pane','list','--session',s['session'],'--json')['result'][0]['id']==s['pane']
 payload='literal $HOME "quotes" \\ žółć 🧪\t\n';file=root/'utf8.txt';file.write_text(payload)
 run('pane','send-text','--id',s['pane'],'--file',str(file),'--json')
 spike.eventually(lambda:(root/'input.bin').read_bytes()==payload.encode())
 run('pane','send-key','--id',s['pane'],'--key','Enter','--json')
 spike.eventually(lambda:(root/'input.bin').read_bytes()==payload.encode()+b'\r')
 run('pane','capture','--id',s['pane'],'--lines','250','--json')
 run('pane','wait','--id',s['pane'],'--contains','READY','--timeout-ms','1000','--json')
 run('pane','send-text','--text','must not route by focus','--json',ok=False)
 run('pane','send-text','--id',s['pane'],'--text','x','--file',str(file),ok=False)
 error=run('pane','capture','--id','pane_'+'0'*32,ok=False);assert error['error']['code']=='STALE_ID'
 run('pane','list','--session',s['session'],'--unknown','x',ok=False)
 run('rdp','open',ok=False)
 error=run('rdp','open','--profile','endpoint_'+'0'*32,ok=False);assert error['error']['code']=='GUI_UNAVAILABLE'
 print('NAMED_CLI PASS',root)
 (root/'acceptance.json').write_text(json.dumps({'status':'PASS','literal_file_input':True,'separate_enter':True,'missing_target_rejected':True,'stale_id_rejected':True,'unknown_flags_rejected':True,'rdp_unavailable_explicit':True},indent=2)+'\n')
finally:
 if server.poll() is None:
  try:
   with spike.Client(sock) as rpc:rpc.call('daemon.stop')
  except Exception:server.terminate()
  server.wait(timeout=8)
 log.close()
