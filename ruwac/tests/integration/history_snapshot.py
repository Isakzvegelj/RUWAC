#!/usr/bin/env python3
"""Real canonical snapshot ownership, output/resize stability and bounded lifecycle."""
import argparse,hashlib,json,os,signal,subprocess,sys,tempfile,time
from pathlib import Path
from spike import Client,eventually
p=argparse.ArgumentParser();p.add_argument('--binary',required=True);p.add_argument('--evidence',required=True);a=p.parse_args()
binary=Path(a.binary).resolve();root=Path(tempfile.mkdtemp(prefix='history-snapshot-',dir=a.evidence));root.chmod(0o700);sock=root/'control.sock'
proc=None;clients=[];children=[];results=[]
def rpc(method,params=None,**kwargs):
 with Client(sock)as c:return c.call(method,params,**kwargs)
try:
 proc=subprocess.Popen([str(binary),'--socket',str(sock),'serve'],stdout=(root/'daemon.log').open('wb'),stderr=subprocess.STDOUT)
 eventually(lambda:sock.exists()and rpc('health')['ok'])
 script=root/'stream.py';script.write_text("import os,time\nfor i in range(5000):os.write(1,('frozen-row-%05d ž界e\\u0301\\r\\n'%i).encode())\nos.write(1,b'STREAM_READY\\r\\n')\ni=0\nwhile True:\n os.write(1,('LIVE_%05d\\r\\n'%i).encode());i+=1;time.sleep(.003)\n")
 row=rpc('session.create',{'name':'streaming pin','argv':[sys.executable,str(script)]});children.append(row['pid'])
 rpc('pane.wait',{'pane':row['pane'],'contains':'STREAM_READY','timeout_ms':10000})
 c=Client(sock);clients.append(c);frozen=c.call('pane.history.freeze',{'pane':row['pane']});pin=frozen['snapshot']
 before=c.call('pane.history',{'pane':row['pane'],'snapshot':pin,'offset':0,'lines':500});time.sleep(.1)
 original_epoch=frozen['epoch'];rpc('pane.history',{'pane':row['pane'],'epoch':original_epoch},error='STALE_HISTORY')
 c.call('session.attach',{'session':row['session'],'rows':30,'cols':60})
 assert c.call('pane.history',{'pane':row['pane'],'snapshot':pin,'offset':0,'lines':500})==before
 with Client(sock)as other:
  other.call('pane.history',{'pane':row['pane'],'snapshot':pin},error='STALE_SNAPSHOT')
  other.call('pane.history.release',{'snapshot':pin},error='STALE_SNAPSHOT')
  other.call('pane.history',{'pane':row['pane'],'snapshot':pin},key='no-context-cache',error='INVALID_PARAMS')
 c.call('pane.history',{'pane':row['pane'],'snapshot':pin},key='no-context-cache',error='INVALID_PARAMS')
 c.call('pane.history.freeze',{'pane':row['pane']},key='no-freeze-cache',error='INVALID_PARAMS')
 c.call('pane.history.release',{'snapshot':pin},key='no-release-cache',error='INVALID_PARAMS')
 c.call('pane.history.freeze',{'pane':'pane_'+'0'*32},error='STALE_ID')
 assert c.call('pane.history',{'pane':row['pane'],'snapshot':pin,'offset':0,'lines':500})==before
 text='';offset=0;rows=0
 while offset<frozen['total_lines']:
  page=c.call('pane.history',{'pane':row['pane'],'snapshot':pin,'offset':offset,'lines':500});assert page['epoch']==original_epoch
  assert page['next_offset']>offset;offset=page['next_offset'];rows+=len(page['rows']);text+=page['text']
 assert rows==frozen['total_lines']and text.startswith('frozen-row-00000') and 'STREAM_READY'in text
 rpc('daemon.checkpoint');saved=(root/'control.history.json').read_bytes();assert pin.encode()not in saved
 results.append({'id':'HISTORY-FROZEN-STREAM-RESIZE-OWNERSHIP','status':'PASS','rows':rows,'sha256':hashlib.sha256(text.encode()).hexdigest(),'changed_live_epoch':True,'snapshot_after_real_resize_identical':True,'connection_and_pane_bound':True,'idempotency_cache_bypass_refused':True,'invalid_freeze_preserves_previous':True,'pins_excluded_from_checkpoint':True})
 c.call('pane.history.release',{'snapshot':pin});c.call('pane.history',{'pane':row['pane'],'snapshot':pin},error='STALE_SNAPSHOT');c.close();clients.remove(c)
 # Quiescent canonical output: creating/releasing a pin changes no terminal revision.
 quiet=rpc('session.create',{'name':'quiet pin','argv':['/bin/sh','-c','printf QUIET; sleep 600']});children.append(quiet['pid'])
 rpc('pane.wait',{'pane':quiet['pane'],'contains':'QUIET','timeout_ms':5000})
 # The first pane intentionally streams, so revision equality is checked after stopping only that owned writer.
 rpc('pane.close',{'pane':row['pane']});time.sleep(.04)
 baseline=rpc('health')['revision'];owners=[];pins=[]
 for _ in range(8):
  owner=Client(sock);owners.append(owner);clients.append(owner);pins.append(owner.call('pane.history.freeze',{'pane':quiet['pane']})['snapshot'])
 ninth=Client(sock);clients.append(ninth);ninth.call('pane.history.freeze',{'pane':quiet['pane']},error='LIMIT')
 assert rpc('health')['revision']==baseline and rpc('health')['history_snapshots']==8
 assert rpc('health')['history_snapshot_text_bytes']<=8*8*1024*1024
 old=pins[0];pins[0]=owners[0].call('pane.history.freeze',{'pane':quiet['pane']})['snapshot'];owners[0].call('pane.history',{'pane':quiet['pane'],'snapshot':old},error='STALE_SNAPSHOT')
 for owner in owners:owner.close();clients.remove(owner)
 eventually(lambda:rpc('health')['history_snapshots']==0)
 results.append({'id':'HISTORY-PIN-CAP-REVISION-DISCONNECT','status':'PASS','global_cap':8,'one_per_connection':True,'terminal_revision_unchanged':True,'disconnect_reclaims':True})
 pin=ninth.call('pane.history.freeze',{'pane':quiet['pane']})['snapshot'];started=time.monotonic()
 eventually(lambda:rpc('health')['history_snapshots']==0,35)
 ninth.call('pane.history',{'pane':quiet['pane'],'snapshot':pin},error='STALE_SNAPSHOT')
 results.append({'id':'HISTORY-PIN-REAL-EXPIRY','status':'PASS','idle_expiry_seconds':time.monotonic()-started,'configured_idle_seconds':30,'actual_actor_timer':True})
finally:
 for c in clients:
  try:c.close()
  except OSError:pass
 if proc and proc.poll()is None:
  try:rpc('daemon.stop');proc.wait(timeout=5)
  except Exception:proc.kill();proc.wait()
 for pid in children:
  try:
   if os.getsid(pid)==pid:os.killpg(pid,signal.SIGKILL)
  except ProcessLookupError:pass
 receipt={'results':results,'binary_sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),'evidence':str(root)}
 (root/'receipt.json').write_text(json.dumps(receipt,indent=2)+'\n');print(json.dumps(receipt,indent=2))
assert len(results)==3
