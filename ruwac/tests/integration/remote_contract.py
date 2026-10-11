#!/usr/bin/env python3
"""SSH subprocess/framing negatives. Synthetic peers; real SSH is tested separately."""
import argparse,json,os,subprocess,tempfile,time
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('--binary',required=True);p.add_argument('--evidence',type=Path,required=True);a=p.parse_args();root=Path(tempfile.mkdtemp(prefix='remote-contract-',dir=a.evidence));root.chmod(0o700);bin=root/'bin';bin.mkdir()
peer=bin/'ssh';peer.write_text('''#!/usr/bin/env python3
import json,os,sys,time
from pathlib import Path
args=sys.argv[1:];data=json.load(sys.stdin);Path(os.environ['PEER_RECEIPT']).write_text(json.dumps({'args':args,'request':data}))
mode=os.environ['PEER_MODE'];value={'version':0,'id':data['id'],'result':{'ok':True}}
if mode=='wrong-id':value['id']='not-requested'
if mode=='wrong-version':value['version']=99
if mode=='error':value.pop('result');value['error']={'code':'STALE_ID','message':'synthetic stale target'}
if mode=='oversize':sys.stdout.write('x'*(1048576+100));sys.stdout.flush();time.sleep(4);sys.exit()
if mode=='invalid-json':print('not-json');sys.exit()
if mode=='eof-live':os.close(1);time.sleep(10);sys.exit()
if mode=='silent-live':time.sleep(90);sys.exit()
print(json.dumps(value))
if mode in ['exit-failed','error']:sys.exit(19)
''');peer.chmod(0o700);receipt=root/'peer.json';env={**os.environ,'PATH':str(bin)+os.pathsep+os.environ['PATH'],'PEER_RECEIPT':str(receipt)}
base=[a.binary,'--host','authorized-alias','--remote-binary',"remote ' ; $(literal)",'--socket',"/private/socket ' ; $(literal)"]
params={'pane':'pane_'+'1'*32,'text':'REQUEST-STDIN-ONLY ž🦀 $HOME $(literal)'}
results=[]
for mode in ['ok','wrong-id','wrong-version','error','oversize','invalid-json','exit-failed','eof-live','silent-live']:
 started=time.monotonic();r=subprocess.run(base+['rpc','pane.sendText','--params',json.dumps(params)],env={**env,'PEER_MODE':mode},capture_output=True,timeout=(80 if mode=='silent-live' else 8));elapsed=time.monotonic()-started
 assert (r.returncode==0)==(mode=='ok'),(mode,r.stdout,r.stderr)
 if mode=='error':assert json.loads(r.stdout)['error']['code']=='STALE_ID'
 d=json.loads(receipt.read_text());assert d['request']['params']==params
 assert params['text']not in ' '.join(d['args'])
 import shlex
 assert shlex.split(d['args'][-1])==["remote ' ; $(literal)",'--socket',"/private/socket ' ; $(literal)",'request']
 assert all(flag in d['args']for flag in ['StrictHostKeyChecking=yes','BatchMode=yes','ForwardAgent=no','ClearAllForwardings=yes'])
 if mode=='eof-live':assert elapsed<3.5,elapsed
 if mode=='silent-live':assert 69<=elapsed<75,elapsed
 results.append({'case':mode,'status':'PASS','elapsed_ms':round(elapsed*1000,2)})
for args in [['--host','-oProxyCommand=bad','health'],['--host','bad host','health'],['--host','alias','--host','second','health'],['--remote-binary','path','health']]:
 receipt.unlink(missing_ok=True);r=subprocess.run([a.binary]+args,env={**env,'PEER_MODE':'ok'},capture_output=True,timeout=5);assert r.returncode and not receipt.exists(),args
results.append({'case':'global-option-validation','status':'PASS','no_ssh_invocation':True})
r=subprocess.run([a.binary,'--host','authorized-alias','rpc','health'],env={**env,'PEER_MODE':'ok','RUWAC_SOCKET':'/local-only/socket'},capture_output=True,timeout=8);assert r.returncode==0
assert __import__('shlex').split(json.loads(receipt.read_text())['args'][-1])==['ruwac','request']
results.append({'case':'remote-default-socket','status':'PASS','local_environment_not_forwarded':True})

(root/'receipt.json').write_text(json.dumps({'cases':results,'qualification':'synthetic SSH subprocess contract; does not establish remote login'},indent=2)+'\n');print('REMOTE_CONTRACT PASS',root)
