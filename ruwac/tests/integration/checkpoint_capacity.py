#!/usr/bin/env python3
"""Real dense dual-buffer PTYs in a disposable memory-limited user service."""
import argparse,hashlib,json,os,signal,subprocess,sys,tempfile,time,uuid
from pathlib import Path
from spike import Client,eventually
p=argparse.ArgumentParser();p.add_argument('--binary',required=True);p.add_argument('--evidence',type=Path,required=True);p.add_argument('--panes',type=int,default=128);p.add_argument('--memory-mib',type=int,default=1536);a=p.parse_args()
assert 1<=a.panes<=128 and 512<=a.memory_mib<=2048
root=Path(tempfile.mkdtemp(prefix='checkpoint-capacity-',dir=a.evidence));root.chmod(0o700);sock=root/'control.sock';state=root/'state.json';unit='ruwac-checkpoint-fixture-'+uuid.uuid4().hex;wrapper=None;log=None;results=[];panes=[];metrics={};start_time=time.monotonic()
app=root/'app.py';app.write_text('''import os,tty
# Each visible cell originates from a real PTY application and canonical parser.
tty.setraw(0)
def paint(label):
 os.write(1,b"\\x1b[?7l")
 for r in range(1,101):
  color=(r%32)+1
  line=f"{label}-{r:03} "+"Že\\u0301界Aa"*40
  os.write(1,(f"\\x1b[{r};1H\\x1b[1;38;2;{color};2;3m\\x1b[48;2;4;{color};6m"+line).encode())
 os.write(1,("\\x1b[1;1H"+label+"-READY").encode())
paint('PRIMARY')
os.write(1,b"\\x1b[?1049h")
paint('ALT')
while os.read(0,1):pass
''')
def service(*args,check=True):return subprocess.run(['systemctl','--user',*args],capture_output=True,text=True,timeout=30,check=check)
def property(name):
 value=service('show',unit,'-p',name,'--value').stdout.strip()
 try:return int(value)
 except ValueError:return None
def rpc(method,q=None):
 with Client(sock)as c:return c.call(method,q)
def healthy():
 try:return rpc('health')
 except (OSError,EOFError):return None
def launch():
 global wrapper,log
 log=(root/('daemon-'+str(len(results))+'.log')).open('wb')
 env=[]
 for key,value in [('RUWAC_STATE_FILE',state),('RUWAC_SCROLLBACK_LINES','2000'),('RUWAC_SCROLLBACK_MIB','8'),('HOME',root/'home'),('XDG_CONFIG_HOME',root/'config'),('XDG_DATA_HOME',root/'data'),('XDG_CACHE_HOME',root/'cache')]:
  if key in ['HOME','XDG_CONFIG_HOME','XDG_DATA_HOME','XDG_CACHE_HOME']:Path(value).mkdir(mode=0o700,exist_ok=True)
  env.append('--setenv='+key+'='+str(value))
 argv=['systemd-run','--user','--quiet','--collect','--wait','--pipe','--service-type=exec','--unit='+unit,'-p',f'MemoryMax={a.memory_mib}M','-p','MemorySwapMax=0','-p','RuntimeMaxSec=240',*env,str(Path(a.binary).resolve()),'--socket',str(sock),'serve']
 wrapper=subprocess.Popen(argv,stdin=subprocess.DEVNULL,stdout=log,stderr=subprocess.STDOUT)
 def ready():
  if wrapper.poll()is not None:raise RuntimeError('Owned memory-limited daemon failed to start')
  return healthy()
 eventually(ready,20)
 assert property('MemoryMax')==a.memory_mib*1024*1024
def digest(pane,buffer):
 r=rpc('pane.capture',{'pane':pane,'lines':2000,'buffer':buffer});return hashlib.sha256(r['text'].encode()).hexdigest()
def record(name,**details):results.append({'id':name,'status':'PASS',**details});print(name,'PASS',flush=True)
try:
 launch()
 session=rpc('session.create',{'name':'Dense checkpoint capacity','rows':100,'cols':200,'argv':[sys.executable,str(app)]});panes.append(session['pane'])
 for index in range(a.panes):
  if index:panes.append(rpc('window.create',{'session':session['session'],'argv':[sys.executable,str(app)]})['pane'])
  pane=panes[-1];eventually(lambda:'ALT-READY'in rpc('pane.capture',{'pane':pane})['text'],20)
  assert 'PRIMARY-READY'in rpc('pane.capture',{'pane':pane,'buffer':'primary'})['text']
  if(index+1)%32==0:print('READY',index+1,flush=True)
 original={pane:{buffer:digest(pane,buffer)for buffer in ['active','primary']}for pane in panes}
 tick=time.monotonic();saved=rpc('daemon.checkpoint');metrics['explicit_checkpoint_ms']=(time.monotonic()-tick)*1000
 metrics['file_bytes']=state.stat().st_size;metrics['memory_peak_bytes']=property('MemoryPeak');metrics['memory_current_bytes']=property('MemoryCurrent')
 assert metrics['file_bytes']<128*1024*1024
 stored=json.loads(state.read_bytes());assert stored['schema']==4 and len(stored['panes'])==a.panes
 assert all(p['terminal']['grid']['cell_encoding']=='palette-v1' and p['terminal']['primary']['grid']['cell_encoding']=='palette-v1'for p in stored['panes'])
 assert all(isinstance(c[2],list) and len(c[2])==3 and isinstance(c[3],list) and len(c[3])==3 for p in stored['panes']for g in [p['terminal']['grid'],p['terminal']['primary']['grid']]for c in g['cell_palette'])
 def images(value):
  return {p['id']:{name:hashlib.sha256(json.dumps(p['terminal'][name]['grid'] if name=='primary' else p['terminal']['grid'],sort_keys=True,ensure_ascii=False).encode()).hexdigest()for name in ['active','primary']}for p in value['panes']}
 before_images=images(stored)
 assert all(p['terminal']['rows']==100 and p['terminal']['cols']==200 for p in stored['panes'])
 record('CHECKPOINT-DENSE-DUAL-CAPACITY',panes=a.panes,dimensions=[100,200],all_cells_colored=True,Unicode_combining_wide=True,unchanged_file_limit=True)
 # Kill only this service's main daemon. systemd reaps its owned PTY children.
 pid=property('MainPID');assert pid and Path(f'/proc/{pid}').stat().st_uid==os.geteuid();os.kill(pid,signal.SIGKILL);wrapper.wait(timeout=30);wrapper=None;log.close();log=None
 eventually(lambda:service('show',unit,'-p','LoadState','--value',check=False).stdout.strip() in ['', 'not-found'],15)
 tick=time.monotonic();launch();metrics['restart_to_health_ms']=(time.monotonic()-tick)*1000
 for pane in panes:
  assert {buffer:digest(pane,buffer)for buffer in ['active','primary']}==original[pane]
 rows=rpc('pane.list',{'session':session['session']});assert len(rows)==a.panes and all(r['ended']and r['restored']and r['pid']is None for r in rows)
 rpc('daemon.checkpoint');assert images(json.loads(state.read_bytes()))==before_images
 record('CHECKPOINT-DENSE-SIGKILL-RECOVERY',canonical_cell_styles_exact=True,both_buffers_exact=True,ended_without_process_resurrection=True,panes=a.panes)
finally:
 service('stop',unit,check=False)
 if wrapper is not None:
  try:wrapper.wait(timeout=30)
  except subprocess.TimeoutExpired:wrapper.kill();wrapper.wait(timeout=5)
 if log is not None:log.close()
 metrics['elapsed_ms']=(time.monotonic()-start_time)*1000
 receipt={'cases':results,'metrics':metrics,'sha256':hashlib.sha256(Path(a.binary).read_bytes()).hexdigest(),'scope':'Real owned PTYs and daemon SIGKILL at dense maximum grid dimensions; memory-limited disposable user service. No owner processes/configuration; not full scrollback saturation, arbitrary per-cell entropy, physical input or idle performance qualification','memory_limit_mib':a.memory_mib,'evidence':str(root)}
 (root/'receipt.json').write_text(json.dumps(receipt,indent=2)+'\n');(a.evidence/'checkpoint-capacity.json').write_text(json.dumps(receipt,indent=2)+'\n');print(json.dumps(receipt,indent=2))
assert len(results)==2
