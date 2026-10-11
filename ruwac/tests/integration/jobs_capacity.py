#!/usr/bin/env python3
"""Real 128-pane metadata payload, wire framing and native all-page acceptance."""
import argparse,hashlib,json,os,socket,struct,subprocess,tempfile,time,uuid
from pathlib import Path
from spike import Client,eventually,exact
p=argparse.ArgumentParser();p.add_argument('--binary',required=True);p.add_argument('--gui');p.add_argument('--evidence',type=Path,required=True);a=p.parse_args()
root=Path(tempfile.mkdtemp(prefix='jobs-capacity-',dir=a.evidence));root.chmod(0o700);sock=root/'control.sock';results=[];daemon=None;gui=None;seq=0
# Keep the large fixture out of history checkpoints; persistence is tested separately.
env={**os.environ,'RUWAC_STATE_FILE':str(root/'state.json'),'GTK_A11Y':'none','GSETTINGS_BACKEND':'memory'}
for key,name in [('HOME','home'),('XDG_CONFIG_HOME','config'),('XDG_DATA_HOME','data'),('XDG_CACHE_HOME','cache')]:env[key]=str(root/name);Path(env[key]).mkdir(mode=0o700)
def rpc(m,p=None,**kw):
 with Client(sock)as c:return c.call(m,p,**kw)
def wire(client,offset):
 request={'version':0,'id':uuid.uuid4().hex,'method':'job.list','params':{'offset':offset,'limit':100}};data=json.dumps(request).encode();client.s.sendall(struct.pack('!I',len(data))+data);n=struct.unpack('!I',exact(client.s,4))[0];assert 0<n<=1024*1024
 reply=json.loads(exact(client.s,n));assert reply['id']==request['id'] and 'error' not in reply,reply;return reply['result'],n
try:
 daemon=subprocess.Popen([a.binary,'--socket',str(sock),'serve'],env=env,stdout=(root/'daemon.log').open('w'),stderr=subprocess.STDOUT);eventually(lambda:sock.exists())
 cwd=root
 for _ in range(16):cwd=cwd/('🧭'*60);cwd.mkdir()
 label='🧭'*256;argv=['/bin/sh','-c','cd "$1"; echo READY; exec sleep 300','generated-argument-not-metadata',str(cwd)]
 s=rpc('session.create',{'name':label,'argv':argv});panes=[s['pane']];last=s
 for i in range(127):
  last=rpc('window.create',{'session':s['session'],'name':('Capacity last target '+label if i==126 else label),'argv':argv});panes.append(last['pane'])
 rpc('window.rename',{'window':s['window'],'name':label});rpc('window.select',{'window':s['window']})
 # Confirm actual processes entered the real long directory before measuring.
 eventually(lambda:rpc('job.query',{'pane':last['pane']})['command']=='sleep',10)
 queries=[rpc('job.query',{'pane':pane}) for pane in panes];assert all(r['command']=='sleep' for r in queries)
 original_pids={r['id']:r['pid'] for r in queries};aggregate=len(json.dumps(queries,ensure_ascii=False,separators=(',',':')).encode());assert aggregate>1024*1024,aggregate
 revision=rpc('health')['revision'];offset=0;seen=[];sizes=[];times=[];membership=None
 with Client(sock)as client:
  for _ in range(128):
   tick=time.monotonic();page,size=wire(client,offset);times.append((time.monotonic()-tick)*1000);sizes.append(size)
   assert page['total']==128 and page['offset']==offset and page['page_byte_limit']==65536 and size<=65536+256,page
   if membership is None:membership=page['membership']
   assert page['membership']==membership
   for row in page['jobs']:
    assert 'generated-argument-not-metadata' not in json.dumps(row);seen.append(row['id'])
   next_offset=page['next_offset']
   if next_offset is None:break
   assert next_offset>offset;offset=next_offset
  else:raise AssertionError('bounded cursor did not finish')
  assert client.call('health')['revision']==revision
 assert set(seen)==set(panes) and len(seen)==128 and len(sizes)>2
 results.append({'id':'JOBS-REAL-128-WIRE-PAGES','status':'PASS','panes':128,'unpaged_row_bytes':aggregate,'transport_pages':len(sizes),'largest_reply_bytes':max(sizes),'page_latency_ms':times,'readonly_revision':True,'arguments_excluded':True})
 if a.gui:
  gui=subprocess.Popen([a.gui,'--socket',str(sock),'--session',s['session'],'--fixture-dir',str(root)],env=env,stdout=(root/'gui.log').open('w'),stderr=subprocess.STDOUT)
  def command(action,**kw):
   global seq
   seq+=1;tmp=root/'request.tmp';tmp.write_text(json.dumps({'seq':seq,'action':action,**kw}));tmp.replace(root/'gui-request.json')
   def read():
    try:r=json.loads((root/'gui-response.json').read_text());return r['result']if r['seq']==seq else None
    except (ValueError,OSError):return None
   return eventually(read,20)
  command('inspect');command('jobs',operation='open');state=eventually(lambda:v if len((v:=command('jobs'))['rows'])==50 and v['page_text'].endswith('of 128') else None,30)
  visible=[r['pane'] for r in state['rows']]
  state=command('jobs',page_action='next');assert len(state['rows'])==50;visible.extend(r['pane']for r in state['rows'])
  state=command('jobs',page_action='next');assert len(state['rows'])==28;visible.extend(r['pane']for r in state['rows']);assert len(visible)==128 and set(visible)==set(panes)
  state=command('jobs',page_action='previous');assert len(state['rows'])==50
  command('jobs',query='Capacity last target');state=eventually(lambda:v if len((v:=command('jobs'))['rows'])==1 else None,15);assert state['rows'][0]['pane']==last['pane']
  command('jobs',jump=last['pane']);eventually(lambda:not command('jobs')['open']);eventually(lambda:command('inspect')['target']==last['pane'],15)
  assert rpc('job.query',{'pane':last['pane']})['pid']==original_pids[last['pane']]
  results.append({'id':'JOBS-NATIVE-128-ALL-PAGES','status':'PASS','actual_GTK_rows':128,'visible_pages':[50,50,28],'last_transport_page_search_and_exact_jump':True,'same_child_pid':True,'scope':'Synthetic GTK handlers on owned compositor; not physical input'})
  gui.terminate();gui.wait(timeout=10);gui=None
 page=rpc('job.list',{'limit':1});rpc('pane.close',{'pane':last['pane']});changed=rpc('job.list',{'offset':page['next_offset'],'limit':1});assert changed['membership']!=page['membership'] and changed['total']==127
 rpc('job.query',{'pane':last['pane']},error='STALE_ID')
 results.append({'id':'JOBS-REAL-MEMBERSHIP-CHANGE','status':'PASS','actual_owned_pane_closed':True,'membership_changed':True,'stale_target_rejected':True})
finally:
 if gui is not None and gui.poll()is None:gui.terminate();gui.wait(timeout=10)
 if daemon is not None and daemon.poll()is None:
  try:rpc('daemon.stop');daemon.wait(timeout=10)
  except Exception:daemon.kill();daemon.wait()
 hashes={'ruwac':hashlib.sha256(Path(a.binary).read_bytes()).hexdigest()}
 if a.gui:hashes['ruwac-desktop']=hashlib.sha256(Path(a.gui).read_bytes()).hexdigest()
 receipt={'results':results,'hashes':hashes,'scope':'Actual owned PTYs/Unix wire payloads and synthetic native GTK actions; no owner sessions accessed'};(root/'receipt.json').write_text(json.dumps(receipt,indent=2)+'\n');print(json.dumps(receipt,indent=2),flush=True)
assert len(results)==(3 if a.gui else 2)
