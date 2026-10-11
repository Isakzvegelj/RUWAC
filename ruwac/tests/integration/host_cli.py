#!/usr/bin/env python3
"""Actual authorized SSH host control and remote-PTY attach; private fixture daemon only."""
import argparse,fcntl,hashlib,json,os,pty,select,shlex,struct,subprocess,tempfile,time
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('--binary',required=True);p.add_argument('--host',required=True);p.add_argument('--remote-binary',required=True);p.add_argument('--remote-receiver',required=True);p.add_argument('--remote-evidence',required=True);p.add_argument('--evidence',type=Path,required=True);a=p.parse_args()
root=Path(tempfile.mkdtemp(prefix='host-cli-',dir=a.evidence));root.chmod(0o700)
ssh=['ssh','-o','BatchMode=yes','-o','StrictHostKeyChecking=yes','-o','ConnectTimeout=10',a.host]
def direct(args):return subprocess.check_output(ssh+[shlex.join(args)],timeout=20)
remote=direct(['python3','-c','import tempfile,sys;print(tempfile.mkdtemp(prefix="host-cli-",dir=sys.argv[1]))',a.remote_evidence]).decode().strip()
socket=remote+"/literal ' ; $(not-executed).sock"
base=[a.binary,'--host',a.host,'--remote-binary',a.remote_binary,'--socket',socket]
def call(*args,input=None,ok=True):
 r=subprocess.run(base+list(args),input=input,capture_output=True,timeout=85)
 assert (r.returncode==0)==ok,(args,r.stdout,r.stderr)
 return json.loads(r.stdout)if r.stdout else None
def wait(fn):
 end=time.monotonic()+12
 while time.monotonic()<end:
  v=fn()
  if v:return v
  time.sleep(.03)
 raise AssertionError('remote fixture condition timed out')
results=[];child=None;master=None
try:
 assert call('start')['result']['ready']
 assert not call('start')['result']['start_requested']
 revision=call('health')['result']['revision']
 d=call('describe')['result'];assert d['protocol']==0 and 'describe'in d['schema']['methods'] and d['capabilities']['max_text_bytes']==262144
 assert call('health')['result']['revision']==revision
 results.append({'id':'HOST-START-DESCRIBE','status':'PASS','authenticated_ssh':True,'literal_remote_socket_path':True,'reuse_existing_daemon':True,'describe_does_not_change_revision':True})
 s=call('rpc','session.create','--params',json.dumps({'name':'remote CLI literal','argv':['python3',a.remote_receiver,remote+'/input.bin']}))['result']
 call('pane','wait','--id',s['pane'],'--contains','READY','--timeout-ms','5000')
 payload='literal $HOME $(touch NEVER) `uname` "quotes" \\ ž🦀\n';local=root/'input.txt';local.write_text(payload)
 call('pane','send-text','--id',s['pane'],'--file',str(local))
 wait(lambda:direct(['cat',remote+'/input.bin'])==payload.encode())
 call('pane','send-key','--id',s['pane'],'--key','Enter');wait(lambda:direct(['cat',remote+'/input.bin'])==payload.encode()+b'\r')
 assert call('session','list')['result'][0]['id']==s['session']
 assert call('pane','list','--session',s['session'])['result'][0]['id']==s['pane']
 literal={'version':0,'id':'remote-json-correlation','method':'pane.capture','params':{'pane':s['pane'],'lines':250}}
 assert call('request',input=json.dumps(literal).encode())['id']==literal['id']
 error=call('pane','capture','--id','pane_'+'0'*32,ok=False);assert error['error']['code']=='STALE_ID'
 call('pane','send-text','--text','no implicit target',ok=False)
 call('gui',ok=False)
 results.append({'id':'HOST-LITERAL-CONTROL','status':'PASS','exact_unicode_and_metacharacters':True,'local_file_read_remote_literal_delivery':True,'separate_enter':True,'request_correlation':True,'stale_target_error':True,'missing_target_rejected':True,'remote_gui_rejected':True})
 # A remote CLI running inside the SSH PTY performs real interactive detach/reattach.
 before=call('pane','list','--session',s['session'])['result'][0]['pid']
 for i in range(3):
  master,slave=pty.openpty();fcntl.ioctl(slave,0x5414,struct.pack('HHHH',31,105,0,0))
  child=subprocess.Popen(base+['attach','--session',s['session']],stdin=slave,stdout=slave,stderr=slave);os.close(slave);output=bytearray()
  def drain():
   while select.select([master],[],[],.03)[0]:
    try:data=os.read(master,65536)
    except OSError:break
    if not data:break
    output.extend(data)
   return b'\x1b[H\x1b[2J'in output
  wait(drain);token=f'host-attach-{i}-Ž';os.write(master,token.encode());wait(lambda:token.encode()in direct(['cat',remote+'/input.bin']))
  os.write(master,b'\x01d');child.wait(timeout=15);drain();assert child.returncode==0,bytes(output[-1000:]);os.close(master);master=None;child=None
  pane=call('pane','list','--session',s['session'])['result'][0];assert pane['pid']==before and pane['cols']==105 and pane['rows']==30,pane
  (root/f'attach-{i}.bin').write_bytes(output)
 results.append({'id':'HOST-PTY-PERSISTENCE','status':'PASS','cycles':3,'same_child_pid':True,'unicode_exact':True,'rows':30,'cols':105})
finally:
 if child and child.poll()is None:child.terminate();child.wait(timeout=10)
 if master is not None:os.close(master)
 try:call('rpc','daemon.stop')
 except Exception:pass
 receipt={'cases':results,'host':a.host,'remote_evidence':remote,'binary_sha256':hashlib.sha256(Path(a.binary).read_bytes()).hexdigest(),'transport':'actual existing SSH key and verified host key; generated private remote daemon/socket; no owner sessions touched'}
 (root/'receipt.json').write_text(json.dumps(receipt,indent=2)+'\n');(a.evidence/'host-cli-acceptance.json').write_text(json.dumps(receipt,indent=2)+'\n');print(json.dumps(receipt,indent=2))
