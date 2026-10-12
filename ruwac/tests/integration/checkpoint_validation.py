#!/usr/bin/env python3
"""Actual PTY output must not replace a readable checkpoint with an invalid one."""
import argparse,hashlib,json,os,signal,subprocess,sys,tempfile
from pathlib import Path
from spike import Client,eventually
p=argparse.ArgumentParser();p.add_argument('--binary',required=True);p.add_argument('--evidence',type=Path,required=True);a=p.parse_args();root=Path(tempfile.mkdtemp(prefix='checkpoint-validation-',dir=a.evidence));root.chmod(0o700);sock=root/'control.sock';state=root/'state.json';app=root/'app.py';app.write_text('''import os,tty
tty.setraw(0)
os.write(1,b'READY')
while True:
 b=os.read(0,1)
 if not b:break
 if b==b'p':os.write(1,('e'+'\\u0301'*3000).encode())
 if b==b'c':os.write(1,b'\\x1b[2J\\x1b[HRECOVERED')
''');env={**os.environ,'RUWAC_STATE_FILE':str(state)}
for key,name in [('HOME','home'),('XDG_CONFIG_HOME','config'),('XDG_CACHE_HOME','cache'),('XDG_DATA_HOME','data')]:env[key]=str(root/name);Path(env[key]).mkdir(mode=0o700)
server=None;logs=[];pid=None;results=[]
def rpc(m,q=None,error=None):
 with Client(sock)as c:return c.call(m,q,error=error)
def start(label):
 global server
 f=(root/(label+'.log')).open('wb');logs.append(f);server=subprocess.Popen([a.binary,'--socket',str(sock),'serve'],env=env,stdout=f,stderr=f)
 def ready():
  if server.poll()is not None:raise RuntimeError('Owned daemon exited during startup')
  try:return rpc('health')
  except(OSError,EOFError):return None
 eventually(ready,15)
try:
 start('daemon');s=rpc('session.create',{'name':'Checkpoint validation','argv':[sys.executable,str(app)]});pid=s['pid'];pane=s['pane'];eventually(lambda:'READY'in rpc('pane.capture',{'pane':pane})['text']);rpc('daemon.checkpoint')
 rpc('pane.sendText',{'pane':pane,'text':'p'});eventually(lambda:len(rpc('pane.capture',{'pane':pane})['text'].encode())>5000);eventually(lambda:not rpc('health')['persistence']['pending'])
 prior=state.read_bytes();rpc('daemon.checkpoint',error='CHECKPOINT');assert state.read_bytes()==prior;assert rpc('health')['persistence']['error']
 results.append({'id':'CHECKPOINT-INVALID-LIVE-OUTPUT-PRESERVES-FILE','status':'PASS','real_PTY_combining_glyph':True,'prior_readable_bytes_preserved':True,'error_visible':True})
 # Prove the preserved bytes are loadable by a distinct owned daemon.
 old=root/'preserved.json';old.write_bytes(prior);old.chmod(0o600);probe_sock=root/'probe.sock';f=(root/'probe.log').open('wb');logs.append(f);probe=subprocess.Popen([a.binary,'--socket',str(probe_sock),'serve'],env={**env,'RUWAC_STATE_FILE':str(old)},stdout=f,stderr=f)
 try:
  def probe_ready():
   if probe.poll()is not None:raise RuntimeError('Preserved checkpoint failed to load')
   try:
    with Client(probe_sock)as c:return c.call('health')
   except(OSError,EOFError):return None
  eventually(probe_ready,15)
  with Client(probe_sock)as c:
   row=c.call('pane.list',{'session':s['session']})[0];assert row['ended']and row['restored']and row['pid']is None;c.call('daemon.stop')
  probe.wait(timeout=10)
 finally:
  if probe.poll()is None:probe.kill();probe.wait(timeout=5)
 rpc('pane.sendText',{'pane':pane,'text':'c'});eventually(lambda:'RECOVERED'in rpc('pane.capture',{'pane':pane})['text']);rpc('daemon.checkpoint');assert state.read_bytes()!=prior and rpc('health')['persistence']['error']is None
 server.kill();server.wait(timeout=10);server=None;start('restored');assert 'RECOVERED'in rpc('pane.capture',{'pane':pane})['text'];rpc('daemon.stop');server.wait(timeout=10);server=None
 results.append({'id':'CHECKPOINT-VALID-OUTPUT-RECOVERS-DURABILITY','status':'PASS','subsequent_valid_write':True,'SIGKILL_restores_recovered_text':True})
finally:
 if server is not None and server.poll()is None:server.kill();server.wait(timeout=5)
 if pid:
  try:
   if str(app).encode()in Path(f'/proc/{pid}/cmdline').read_bytes():os.kill(pid,signal.SIGKILL)
  except FileNotFoundError:pass
 for f in logs:f.close()
 receipt={'cases':results,'sha256':hashlib.sha256(Path(a.binary).read_bytes()).hexdigest(),'scope':'Real owned PTY/daemon and actual file preservation, recovery, SIGKILL; no owner configuration or sessions','evidence':str(root)};(root/'receipt.json').write_text(json.dumps(receipt,indent=2)+'\n');(a.evidence/'checkpoint-validation.json').write_text(json.dumps(receipt,indent=2)+'\n');print(json.dumps(receipt,indent=2))
assert len(results)==2
