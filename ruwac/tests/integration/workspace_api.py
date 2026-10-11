#!/usr/bin/env python3
"""Actual GTK workspace IPC, persistent graphical IDs and permission boundaries.
The RDP view uses a synthetic frame; authenticated RDP is qualified separately.
"""
import argparse,importlib.util,json,os,signal,socket,struct,subprocess,tempfile,time
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('--binary',required=True);p.add_argument('--gui',required=True);p.add_argument('--evidence',type=Path,required=True);a=p.parse_args()
spec=importlib.util.spec_from_file_location('spike',Path(__file__).with_name('spike.py'));spike=importlib.util.module_from_spec(spec);spec.loader.exec_module(spike)
root=Path(tempfile.mkdtemp(prefix='workspace-api-',dir=a.evidence));root.chmod(0o700);env={**os.environ,'RUWAC_GUI_ENTRY':a.gui,'GTK_A11Y':'none','GSETTINGS_BACKEND':'memory'}
for k,d in [('HOME','home'),('XDG_CONFIG_HOME','config'),('XDG_DATA_HOME','data'),('XDG_CACHE_HOME','cache')]:env[k]=str(root/d);Path(env[k]).mkdir(mode=0o700)
sock=root/'daemon.sock';ipc=sock.with_suffix('.gui.sock');seq=0;gui=None;cases=[]
def api(args,ok=True):
 r=subprocess.run([a.binary,'--socket',str(sock),'workspace',*args,'--json'],capture_output=True,text=True,env=env,timeout=7);assert(r.returncode==0)==ok,(args,r.stdout,r.stderr);d=json.loads(r.stdout);assert d['version']==0 and d['id'];return d
def listing():return api(['list'])['result']
def command(action,**kw):
 global seq
 seq+=1;t=root/'gui-request.tmp';t.write_text(json.dumps({'seq':seq,'action':action,**kw}));t.replace(root/'gui-request.json')
 def read():
  try:r=json.loads((root/'gui-response.json').read_text());return r['result']if r['seq']==seq else None
  except (FileNotFoundError,ValueError):return None
 return spike.eventually(read,10)
def launch(name):
 global gui,seq
 seq=0;gui=subprocess.Popen([a.binary,'--socket',str(sock),'gui','--fixture-dir',str(root)],env=env,stdout=(root/(name+'.log')).open('wb'),stderr=subprocess.STDOUT);command('inspect');spike.eventually(lambda:ipc.exists()and listing(),10)
def close():command('workspace-close');gui.wait(timeout=10)
def raw(req):
 with spike.Client(ipc)as c:
  data=json.dumps(req).encode();c.s.sendall(struct.pack('!I',len(data))+data);return json.loads(spike.exact(c.s,struct.unpack('!I',spike.exact(c.s,4))[0]))
def focus(vid):api(['focus','--id',vid]);return spike.eventually(lambda:listing()['focused_view']==vid and listing()['input_focus_view']==vid)
def all_ids(state):return {v['id']for v in state['views']}
def pids():
 with spike.Client(sock)as c:return {p['id']:p['pid']for s in c.call('session.list')for p in c.call('pane.list',{'session':s['id']})}
try:
 missing=api(['list'],False);assert missing['error']['code']=='GUI_UNAVAILABLE' and not sock.exists() and not ipc.exists()
 ipc.write_text('unrelated path');ipc.chmod(0o600);assert api(['list'],False)['error']['code']=='GUI_UNAVAILABLE' and ipc.read_text()=='unrelated path';ipc.unlink();cases.append({'id':'GUI-IPC-NO-AUTOSTART','status':'PASS','unsafe_path_preserved':True})
 # A live listener with a full backlog must be preserved without blocking GTK startup.
 blocked=root/'blocked';blocked.mkdir(mode=0o700);blocked_sock=blocked/'daemon.sock';blocked_ipc=blocked_sock.with_suffix('.gui.sock');listener=socket.socket(socket.AF_UNIX);fillers=[];blocked_gui=None
 blocked_env={**env}
 for k,d in [('HOME','home'),('XDG_CONFIG_HOME','config'),('XDG_DATA_HOME','data'),('XDG_CACHE_HOME','cache')]:blocked_env[k]=str(blocked/d);Path(blocked_env[k]).mkdir(mode=0o700)
 subprocess.run([a.binary,'--socket',str(blocked_sock),'start'],check=True,capture_output=True,env=blocked_env)
 try:
  listener.bind(str(blocked_ipc));blocked_ipc.chmod(0o600);listener.listen(1);inode=blocked_ipc.stat().st_ino
  for _ in range(2):
   s=socket.socket(socket.AF_UNIX);s.settimeout(1);s.connect(str(blocked_ipc));fillers.append(s)
  blocked_gui=subprocess.Popen([a.gui,'--socket',str(blocked_sock),'--fixture-dir',str(blocked)],env=blocked_env,stdout=(blocked/'gui.log').open('wb'),stderr=subprocess.STDOUT)
  (blocked/'gui-request.json').write_text(json.dumps({'seq':1,'action':'inspect'}));spike.eventually(lambda:(blocked/'gui-response.json').exists(),4);assert json.loads((blocked/'gui-response.json').read_text())['result']['tabs']==1
  assert blocked_ipc.stat().st_ino==inode and listener.getsockopt(socket.SOL_SOCKET,socket.SO_ACCEPTCONN)==1
  cases.append({'id':'GUI-IPC-LIVE-BACKLOG-PRESERVED','status':'PASS','desktop_startup_not_blocked':True,'existing_listener_unchanged':True})
 finally:
  if blocked_gui is not None and blocked_gui.poll()is None:blocked_gui.terminate();blocked_gui.wait(timeout=10)
  for s in fillers:s.close()
  listener.close();blocked_ipc.unlink(missing_ok=True);subprocess.run([a.binary,'--socket',str(blocked_sock),'rpc','daemon.stop'],capture_output=True,timeout=8)
 launch('initial');state=listing();assert len(state['views'])==1 and ipc.stat().st_mode&0o777==0o600
 samples=[]
 for _ in range(30):
  started=time.perf_counter();listing();samples.append((time.perf_counter()-started)*1000)
 samples.sort();cases.append({'id':'GUI-WORKSPACE-API-LATENCY','status':'PASS','trials':30,'median_ms':__import__('statistics').median(samples),'p95_ms':samples[28],'max_ms':samples[-1],'scope':'CLI spawn plus authenticated socket and actual GTK dispatch; isolated compositor; not a matched tmux or idle qualification'})
 first=state['views'][0]['id'];tab0=state['tabs'][0]['id'];assert api(['focus','--id',first],False)['error']['code']=='PERMISSION_DENIED'
 assert not api(['describe'])['result']['permissions']['focus_control'];assert command('workspace-permission',enabled=True)['native_checkbutton']
 for n in [2,3]:command('new-terminal');spike.eventually(lambda:len(listing()['views'])==n)
 terminals=listing()['views'];terminal_second=terminals[1]['id'];before_pids=pids();command('rdp-open');rdp=next(v['id']for v in listing()['views']if v['kind']=='rdp')
 command('files-open',profile={'id':'fixture','name':'Files','group':'Fixture','protocol':'SSH','server':'127.0.0.1:1','username':'fixture','secret_reference':'fixture','favorite':False,'last_used':0,'tags':[]},path='/');file_id=next(v['id']for v in listing()['views']if v['kind']=='files');baseline_ids=all_ids(listing());assert len(baseline_ids)==5
 command('mixed-join',tab=3,vertical=True);command('mixed-join',tab=1,vertical=False);assert all_ids(listing())==baseline_ids;focus(rdp);mixed=listing()['selected_tab'];api(['activate','--id',tab0]);api(['activate','--id',mixed]);assert listing()['focused_view']==rdp
 assert command('views-manage')['clicked'];assert command('views-action',operation='focus',index=2)['clicked'];spike.eventually(lambda:listing()['focused_view']==terminal_second)
 cases.append({'id':'GUI-IDENTITIES-MIXED-FOCUS','status':'PASS','stable_ids_after_reparent':True,'exact_leaf_restored_on_tab_activation':True,'native_focus_button':True,'kinds':['terminal','rdp','files']})
 command('workspace-permission',enabled=False);assert api(['focus','--id',rdp],False)['error']['code']=='PERMISSION_DENIED';assert listing()['focused_view']==terminal_second;command('workspace-permission',enabled=True)
 sentinel='PRIVATE_SENTINEL_DO_NOT_AUDIT'
 requests=[({'version':1,'id':'wrong-version','method':'workspace.list','params':{}},'UNSUPPORTED_VERSION'),({'version':0,'id':'missing-target','method':'workspace.focus','params':{}},'INVALID_TARGET'),({'version':0,'id':'clipboard','method':'clipboard.read','params':{}},'UNSUPPORTED_METHOD'),({'version':0,'id':'invalid','method':'workspace.focus','params':{'id':sentinel}},'STALE_TARGET'),({'version':0,'id':'extra','method':'workspace.list','params':{'password':sentinel}},'INVALID_REQUEST')]
 for req,code in requests:
  response=raw(req);assert response['id']==req['id']and response['error']['code']==code,response
 assert sentinel not in(root/'initial.log').read_text()
 command('ipc-delay',ms=3200);started=time.monotonic();timeout=api(['focus','--id',rdp],False);assert timeout['error']['code']=='GUI_TIMEOUT' and time.monotonic()-started<3;time.sleep(1.3);assert listing()['focused_view']==terminal_second
 with socket.socket(socket.AF_UNIX)as slow:
  slow.settimeout(4);slow.connect(str(ipc));slow.sendall(b'\0\0');time.sleep(2.2);header=slow.recv(4);assert len(header)==4
 assert listing()['focused_view']==terminal_second
 # Oversized frames and excess stalled peers fail without stalling the GTK loop.
 with socket.socket(socket.AF_UNIX)as oversized:
  oversized.settimeout(3);oversized.connect(str(ipc));oversized.sendall(struct.pack('!I',1024*1024+1));size=struct.unpack('!I',spike.exact(oversized,4))[0];assert json.loads(spike.exact(oversized,size))['error']['code']=='INVALID_REQUEST'
 held=[]
 try:
  for _ in range(8):
   s=socket.socket(socket.AF_UNIX);s.settimeout(2);s.connect(str(ipc));held.append(s)
  time.sleep(.2);ticks=command('inspect')['ticks']
  with socket.socket(socket.AF_UNIX)as excess:
   excess.settimeout(1);excess.connect(str(ipc));assert excess.recv(1)==b''
  assert command('inspect')['ticks']>ticks
 finally:
  for s in held:s.close()
 spike.eventually(lambda:listing(),5)
 cases.append({'id':'GUI-IPC-PERMISSIONS-BOUNDS','status':'PASS','runtime_revocation':True,'version_correlation':True,'missing_target_and_unknown_clipboard_method_rejected':True,'audit_contains_no_test_secret':True,'expired_queued_focus_not_executed':True,'slow_frame_deadline':True,'oversized_frames_rejected':True,'peer_limit_and_gtk_responsiveness':True})
 command('views-manage');assert command('views-action',operation='move',index=1)['clicked'];state=listing();moved=next(v['tab']for v in state['views']if v['id']==rdp);assert moved!=mixed and all_ids(state)==baseline_ids
 index=next(i for i,t in enumerate(state['tabs'])if t['id']==moved);command('close-tab',tab=index);assert api(['focus','--id',rdp],False)['error']['code']=='STALE_TARGET';assert api(['activate','--id',moved],False)['error']['code']=='STALE_TARGET';assert len(listing()['views'])==4 and pids()==before_pids
 cases.append({'id':'GUI-IDENTITIES-MOVE-CLOSE','status':'PASS','view_identity_preserved_when_tab_changes':True,'closed_view_and_tab_ids_rejected':True,'daemon_pids_preserved':True})
 focus(terminal_second);command('mixed-separate');spike.eventually(lambda:listing()['focused_view']==terminal_second and listing()['input_focus_view']==terminal_second)
 file_tab=next(v['tab']for v in listing()['views']if v['id']==file_id);file_index=next(i for i,t in enumerate(listing()['tabs'])if t['id']==file_tab);command('mixed-join',tab=file_index,vertical=True);spike.eventually(lambda:listing()['focused_view']==terminal_second and listing()['input_focus_view']==terminal_second)
 cases.append({'id':'GUI-IDENTITIES-LAYOUT-FOCUS','status':'PASS','exact_focus_after_join_and_separate':True})
 path=root/'config/RUWAC/workspace.json';spike.eventually(lambda:path.exists()and json.loads(path.read_text()).get('schema')==3,5);saved=listing();time.sleep(.9);gui.kill();gui.wait(timeout=10);launch('after-crash');restored=listing();assert all_ids(restored)==all_ids(saved)and [t['id']for t in restored['tabs']]==[t['id']for t in saved['tabs']]and restored['focused_view']==terminal_second;assert pids()==before_pids;assert not api(['describe'])['result']['permissions']['focus_control']
 assert next(v['kind']for v in restored['views']if v['id']==file_id)=='ended';close();assert not ipc.exists()
 cases.append({'id':'GUI-IDENTITIES-CRASH-RESTORE','status':'PASS','tab_view_ids_and_exact_focus_persisted':True,'stale_socket_recovered':True,'permissions_reset_each_launch':True,'missing_profile_ended_identity_preserved':True,'terminal_pids_preserved':True})
 legacy=json.loads(path.read_text());legacy['schema']=2
 def strip(l):
  l.pop('view_id',None)
  if l['kind']=='split':strip(l['first']);strip(l['second'])
 for t in legacy['tabs']:t.pop('id',None);t.pop('focused_view',None);strip(t['layout'])
 original=json.dumps(legacy,indent=2)+'\n';path.write_text(original);launch('v2-migration');migrated=listing();spike.eventually(lambda:json.loads(path.read_text())['schema']==3,5);backup=path.with_name('workspace-v2.json');assert backup.read_text()==original and backup.stat().st_mode&0o777==0o600;assert pids()==before_pids;close();launch('migration-stable');assert all_ids(listing())==all_ids(migrated);close()
 cases.append({'id':'GUI-IDENTITIES-V2-MIGRATION','status':'PASS','original_backup_0600':True,'migration_ids_stable_on_next_launch':True,'terminal_pids_preserved':True})
finally:
 if gui is not None and gui.poll()is None:
  try:(root/'final-state.json').write_text(json.dumps(listing(),indent=2))
  except Exception:pass
  gui.terminate();gui.wait(timeout=10)
 if sock.exists():subprocess.run([a.binary,'--socket',str(sock),'rpc','daemon.stop'],capture_output=True,timeout=8)
 receipt={'cases':cases,'evidence':str(root),'binary_sha256':__import__('hashlib').sha256(Path(a.binary).read_bytes()).hexdigest(),'gui_sha256':__import__('hashlib').sha256(Path(a.gui).read_bytes()).hexdigest()};(root/'receipt.json').write_text(json.dumps(receipt,indent=2)+'\n');(a.evidence/'workspace-api-acceptance.json').write_text(json.dumps(receipt,indent=2)+'\n');print(json.dumps(receipt,indent=2))
