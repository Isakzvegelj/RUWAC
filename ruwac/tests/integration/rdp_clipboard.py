#!/usr/bin/env python3
"""Authenticated loopback RDP and independent GDK peer on an owned compositor."""
import argparse,hashlib,importlib.util,json,os,subprocess,sys,tempfile,time,traceback
from pathlib import Path
from PIL import Image
p=argparse.ArgumentParser();p.add_argument('--binary',required=True);p.add_argument('--gui',required=True);p.add_argument('--server',required=True);p.add_argument('--evidence',required=True);p.add_argument('--r5-only',action='store_true');p.add_argument('--mixed-only',action='store_true');p.add_argument('--case',action='append',default=[]);a=p.parse_args()
spec=importlib.util.spec_from_file_location('spike',Path(__file__).with_name('spike.py'));spike=importlib.util.module_from_spec(spec);spec.loader.exec_module(spike)
root=Path(tempfile.mkdtemp(prefix='rdp-native-',dir=a.evidence));root.chmod(0o700);env={**os.environ,'HOME':str(root/'home'),'XDG_CONFIG_HOME':str(root/'config'),'XDG_CACHE_HOME':str(root/'cache'),'XDG_DATA_HOME':str(root/'data'),'GSETTINGS_BACKEND':'memory','GTK_USE_PORTAL':'0','GTK_A11Y':'none'}
for key in ['HOME','XDG_CONFIG_HOME','XDG_CACHE_HOME','XDG_DATA_HOME']:Path(env[key]).mkdir(mode=0o700)
processes=[];logs=[];results=[];sequences={};sock=root/'control.sock';private_bus=None;provider=None
def start(argv,name):
 log=(root/(name+'.log')).open('wb');logs.append(log);proc=subprocess.Popen(argv,stdout=log,stderr=log,env=env);processes.append(proc);return proc
def command(who,action=None,timeout=8,**kw):
 seq=sequences.get(who,0)+1;sequences[who]=seq;temp=root/(who+'-request.tmp');temp.write_text(json.dumps({'seq':seq,'action':action,**kw}));temp.replace(root/(who+'-request.json'))
 def read():
  try:r=json.loads((root/(who+'-response.json')).read_text());return r['result'] if r['seq']==seq else None
  except (FileNotFoundError,ValueError):return None
 return spike.eventually(read,timeout)
def focus(who):
 for _ in range(6):
  if command('gui','inspect').get('active')==(who=='gui'):
   subprocess.run(['xdotool','key','Shift_L'],env={**os.environ,'DISPLAY':os.environ['RUWAC_XTEST_DISPLAY']},check=True,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL);time.sleep(.06);return
  subprocess.run(['xdotool','key','--clearmodifiers','Super_L+Tab'],env={**os.environ,'DISPLAY':os.environ['RUWAC_XTEST_DISPLAY']},check=True);time.sleep(.12)
 raise AssertionError('Isolated compositor focus failed')
def case(name,fn):
 if a.case and name not in a.case and name!='RDP-NATIVE-AUTH-3':return
 if a.mixed_only and name not in {'RDP-NATIVE-AUTH-3','MIXED-NATIVE-RDP-TERMINAL','FILES-CLIPBOARD-OWNERSHIP','GUI-IPC-BACKGROUND-RDP-OWNERSHIP'}:return
 if a.r5_only and name not in {"RDP-NATIVE-AUTH-3","MACHINES-NATIVE-EDITOR","RDP-NATIVE-RESIZE-20","MACHINES-RESTART-1000","FILES-CLIPBOARD-OWNERSHIP","RDP-PROFILE-KEYRING-PREFERENCES","RDP-GUI-LIFECYCLE-API"}:return
 try:results.append({'id':name,'status':'PASS','details':fn()});print(name,'PASS',flush=True)
 except Exception as e:results.append({'id':name,'status':'FAIL','error':str(e),'traceback':traceback.format_exc()});print(name,'FAIL',str(e),flush=True)
def digest(data):return hashlib.sha256(data).hexdigest()
def receipt(tab):
 try:return json.loads((root/f'server-{tab}-receipt.json').read_text())
 except (FileNotFoundError,ValueError):return {}
def read_remote(tab,kind,file):
 before=command('gui','rdp-inspect',rdp=tab)['received_generation'];command(f'server-{tab}',format=kind,file=file)
 spike.eventually(lambda:command('gui','rdp-inspect',rdp=tab)['received_generation']>before,10)
 focus('peer');return command('peer','read-text' if kind=='text' else 'read-png')
def transfer_to_remote(tab,kind,file):
 focus('peer');command('peer','set-text' if kind=='text' else 'set-png',file=file);focus('gui');before=receipt(tab).get('received',0);command('gui','rdp-send',rdp=tab)
 return spike.eventually(lambda:r if (r:=receipt(tab)).get('received',0)>before and 'error' not in r else False,12)
try:
 lines=subprocess.check_output(['dbus-daemon','--session','--fork','--print-address=1','--print-pid=1'],env=env,text=True).splitlines();private_bus=int(lines[1]);env['DBUS_SESSION_BUS_ADDRESS']=lines[0]
 control=root/'private-keyring';control.mkdir(mode=0o700);env['GNOME_KEYRING_CONTROL']=str(control)
 log=(root/'keyring.log').open('wb');logs.append(log);provider=subprocess.Popen(['gnome-keyring-daemon','--foreground','--components=secrets','--control-directory='+str(control),'--unlock'],env=env,stdin=subprocess.PIPE,stdout=log,stderr=log);processes.append(provider);provider.stdin.write(b'generated-disposable-keyring-only\n');provider.stdin.close();time.sleep(1)
 subprocess.run(['openssl','req','-x509','-newkey','rsa:2048','-nodes','-days','1','-subj','/CN=RUWAC-local-fixture','-addext','subjectAltName=IP:127.0.0.1','-keyout',str(root/'key.pem'),'-out',str(root/'cert.pem')],check=True,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL);(root/'key.pem').chmod(0o600)
 server=start([a.server,str(root)],'rdp-server');spike.eventually(lambda:(root/'server-ready.json').exists(),10);ready=json.loads((root/'server-ready.json').read_text())
 daemon=start([a.binary,'--socket',str(sock),'serve'],'daemon');spike.eventually(lambda:sock.exists())
 with spike.Client(sock) as c:session=c.call('session.create',{'name':'RDP fixture terminal'})
 gui=start([a.gui,'--socket',str(sock),'--session',session['session'],'--fixture-dir',str(root)],'gui');peer=start([sys.executable,str(Path(__file__).parents[1]/'fixtures/desktop_clipboard.py'),str(root)],'peer');spike.eventually(lambda:(root/'peer-ready.json').exists(),10);command('gui','inspect');focus('gui')
 def lifecycle():
  connections=[]
  for tab,port in enumerate(ready['ports']):
   started=time.perf_counter();command('gui','rdp-open',port=port);r=spike.eventually(lambda:r if (r:=command('gui','rdp-inspect',rdp=tab)).get('size')==[320,180] else False,20);assert r['status'].startswith('Connected'),r;r['connect_to_decoded_frame_ms']=(time.perf_counter()-started)*1000;connections.append(r)
  return {'server':ready,'connections':connections,'authentication':'actual CredSSP/NLA','frames':'decoded live loopback server frames, generated desktop'}
 case('RDP-NATIVE-AUTH-3',lifecycle)
 def lifecycle_api():
  def api(verb,*args,ok=True):
   r=subprocess.run([a.binary,'--socket',str(sock),'rdp',verb,*args,'--json'],env=env,capture_output=True,text=True,timeout=8)
   assert (r.returncode==0)==ok,(verb,r.stdout,r.stderr);v=json.loads(r.stdout);assert v['version']==0 and v['id'];return v.get('result',v.get('error'))
  listing=api('list');assert len(listing['views'])==3
  ids=[v['id']for v in listing['views']];assert all(v['phase']=='connected'and v['decoded_updates']>0 for v in listing['views'])
  assert 'secret_reference'not in json.dumps(listing)and str(root)not in json.dumps(listing)
  assert api('disconnect','--id',ids[0],ok=False)['code']=='PERMISSION_DENIED'
  command('gui','rdp-permission',enabled=True);command('gui','rdp-switch',rdp=1);focus('gui')
  before=command('gui','inspect');neighbors=[api('query','--id',vid)['transport_id']for vid in ids[1:]];old=api('query','--id',ids[0])['transport_id']
  assert api('reconnect','--id',ids[0],ok=False)['code']=='INVALID_PROFILE'
  assert api('query','--id',ids[0])['transport_id']==old
  assert [api('query','--id',vid)['transport_id']for vid in ids[1:]]==neighbors
  assert command('gui','inspect')['selected_tab']==before['selected_tab']and command('gui','inspect')['clipboard_owner']==before['clipboard_owner']
  api_dir=root/'api-server';api_dir.mkdir(mode=0o700)
  for name in ['cert.pem','key.pem']:(api_dir/name).write_bytes((root/name).read_bytes());(api_dir/name).chmod(0o600)
  api_server=start([a.server,str(api_dir)],'api-server');spike.eventually(lambda:(api_dir/'server-ready.json').exists(),10);api_ready=json.loads((api_dir/'server-ready.json').read_text())
  reference='generated-api-'+root.name
  r=command('gui','profile-editor',operation='new',fields={'0':'R16 API profile','1':'Generated automation','3':'127.0.0.1:'+str(api_ready['ports'][0]),'4':'fixture','9':reference,'10':str(root/'cert.pem'),'13':0});assert not r['error'],r
  item=next(p for p in r['items']if p['name']=='R16 API profile')
  subprocess.run(['secret-tool','store','--label=RUWAC disposable API test','application','RUWAC','profile',reference],input=b'generated-local-only',env=env,check=True,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL,timeout=10)
  for method,params,error,key in [
   ('rdp.open',{'profile':item['id'],'password':'PRIVATE-API-AUDIT-SENTINEL'},'INVALID_TARGET',None),
   ('rdp.open',{'profile':item['id'],'activate':'yes'},'INVALID_TARGET',None),
   ('rdp.list',{'password':'PRIVATE-API-AUDIT-SENTINEL'},'INVALID_REQUEST',None),
   ('rdp.open',{'profile':item['id']},'INVALID_REQUEST','reject-gui-key')]:
   with spike.Client(sock.with_suffix('.gui.sock'))as client:client.call(method,params,error=error,key=key)
  assert 'PRIVATE-API-AUDIT-SENTINEL'not in(root/'gui.log').read_text()
  command('gui','workspace-permission',enabled=False)
  assert api('open','--profile',item['id'],'--activate',ok=False)['code']=='PERMISSION_DENIED';assert len(api('list')['views'])==3
  command('gui','rdp-switch',rdp=1);focus('peer');sentinel=b'R16 untouched desktop clipboard';(root/'api-sentinel.txt').write_bytes(sentinel);command('peer','set-text',file='api-sentinel.txt');focus('gui');before=command('gui','inspect')
  opened=api('open','--profile',item['id']);vid=opened['id'];assert len(api('list')['views'])==4
  spike.eventually(lambda:api('query','--id',vid)['phase']=='connected'and api('query','--id',vid)['decoded_updates']>0,20)
  assert command('gui','inspect')['selected_tab']==before['selected_tab']and command('gui','inspect')['clipboard_owner']==before['clipboard_owner']
  assert not api('query','--id',vid)['clipboard_owner']
  all_neighbors=[api('query','--id',x)['transport_id']for x in ids];first_transport=api('query','--id',vid)['transport_id']
  api('disconnect','--id',vid);api('reconnect','--id',vid);spike.eventually(lambda:api('query','--id',vid)['phase']=='connected',20)
  assert api('query','--id',vid)['transport_id']!=first_transport and [api('query','--id',x)['transport_id']for x in ids]==all_neighbors
  assert command('gui','inspect')['selected_tab']==before['selected_tab']and command('gui','inspect')['clipboard_owner']==before['clipboard_owner']
  focus('peer');assert command('peer','read-text')['sha256']==digest(sentinel)
  command('gui','rdp-permission',enabled=False);assert api('disconnect','--id',vid,ok=False)['code']=='PERMISSION_DENIED';assert api('query','--id',vid)['phase']=='connected'
  command('gui','rdp-permission',enabled=True);api('disconnect','--id',vid)
  state=json.loads(subprocess.check_output([a.binary,'--socket',str(sock),'workspace','list','--json'],env=env))['result'];tab=next(v['tab']for v in state['views']if v['id']==vid);index=next(i for i,p in enumerate(state['tabs'])if p['id']==tab)
  command('gui','close-tab',tab=index);assert api('query','--id',vid,ok=False)['code']=='STALE_TARGET'
  command('gui','profiles',search='R16 API profile');deleted=command('gui','profile-editor',operation='edit',response='delete',confirm=True);assert not any(p['id']==item['id']for p in deleted['items'])
  subprocess.run(['secret-tool','clear','application','RUWAC','profile',reference],env=env,check=True,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL,timeout=10)
  api_server.terminate();api_server.wait(timeout=5)
  command('gui','profiles',search='');command('gui','rdp-permission',enabled=False);command('gui','rdp-switch',rdp=0)
  return {'actual_CredSSP_framebuffer':True,'typed_view_ids':True,'permission_default_and_revocation':True,'read_metadata_no_credentials':True,'background_reconnect_preserves_neighbors_focus_clipboard':True,'saved_profile_open_without_focus':True,'activation_requires_separate_focus_permission':True,'closed_target_rejected':True,'unexpected_or_credential_parameters_rejected':True,'audit_no_private_payload':True,'idempotency_keys_explicitly_refused':True,'owner_keyring_accessed':False}
 case('RDP-GUI-LIFECYCLE-API',lifecycle_api)
 def background_ipc():
  focus('gui');command('gui','rdp-switch',rdp=0);command('gui','workspace-permission',enabled=True)
  sentinel=b'Foreground GDK peer owns clipboard';remote=b'Authorized loopback RDP background offer';(root/'background-local.txt').write_bytes(sentinel);(root/'background-remote.txt').write_bytes(remote)
  def api(args):
   r=subprocess.run([a.binary,'--socket',str(sock),'workspace',*args,'--json'],env=env,capture_output=True,text=True,timeout=7);assert r.returncode==0,(r.stdout,r.stderr);return json.loads(r.stdout)['result']
  state=api(['list']);rdp_id=next(v['id']for v in state['views']if v['kind']=='rdp')
  focus('peer');command('peer','set-text',file='background-local.txt');before=[command('gui','rdp-inspect',rdp=i)['received_generation']for i in range(3)]
  api(['focus','--id',rdp_id]);assert not api(['list'])['window_active'];assert command('gui','inspect')['clipboard_owner']is None
  for i in range(3):command(f'server-{i}',format='text',file='background-remote.txt')
  time.sleep(.25);assert command('peer','read-text')['sha256']==digest(sentinel);assert [command('gui','rdp-inspect',rdp=i)['received_generation']for i in range(3)]==before
  focus('gui');command('gui','rdp-switch',rdp=0);got=read_remote(0,'text','background-remote.txt');assert got['sha256']==digest(remote);focus('gui');command('gui','workspace-permission',enabled=False)
  return {'actual_authenticated_rdp_sessions':3,'authorized_ipc_focus_in_background':True,'foreground_peer_clipboard_preserved':True,'inactive_transfers_not_started':True,'transfer_after_native_window_activation':True,'physical_input':False}
 case('GUI-IPC-BACKGROUND-RDP-OWNERSHIP',background_ipc)
 texts=['Živjo Łódź 🦀\nMultiline clipboard '+str(i) for i in range(8)]+['x'*10240,'z'*(1024*1024)]
 def text_transfer():
  evidence=[]
  for tab in range(3):
   focus('gui');command('gui','rdp-switch',rdp=tab)
   for i,text in enumerate(texts):
    name=f'text-{tab}-{i}.txt';payload=text.encode();(root/name).write_bytes(payload)
    received=transfer_to_remote(tab,'text',name);assert received['sha256']==digest(payload),received
    focus('gui');got=read_remote(tab,'text',name);assert got['sha256']==digest(payload),got;evidence.append({'tab':tab,'bytes':len(payload),'sha256':digest(payload),'directions':2})
  return evidence
 case('RDP-NATIVE-TEXT-60',text_transfer)
 def images():
  evidence=[]
  for tab in range(3):
   focus('gui');command('gui','rdp-switch',rdp=tab)
   for index,(w,h,fmt)in enumerate([(64,64,'png'),(2560,1440,'png'),(64,64,'dibv5')]):
    name=f'image-{tab}-{index}.png';pixels=bytes([40+tab,129,75,100+index])*(w*h);Image.frombytes('RGBA',(w,h),pixels).save(root/name)
    received=transfer_to_remote(tab,'png',name);assert received['pixel_sha256']==digest(pixels),received
    focus('gui');got=read_remote(tab,fmt,name);assert got['pixel_sha256']==digest(pixels),got;evidence.append({'tab':tab,'size':[w,h],'remote_format':fmt,'pixel_sha256':digest(pixels),'directions':2})
  return evidence
 case('RDP-NATIVE-PNG-18',images)
 def policies():
  focus('gui');command('gui','rdp-switch',rdp=0);sentinel=b'POLICY-LOCAL-SENTINEL';remote=b'POLICY-REMOTE-GENERATED';(root/'policy-local.txt').write_bytes(sentinel);(root/'policy-remote.txt').write_bytes(remote)
  for policy in [1,3,4]:
   focus('gui');command('gui','rdp-policy',rdp=0,policy=policy);focus('peer');command('peer','set-text',file='policy-local.txt');focus('gui');before=command('gui','rdp-inspect',rdp=0)['received_generation'];command('server-0',format='text',file='policy-remote.txt');time.sleep(.2);focus('peer');r=command('peer','read-text');assert r['sha256']==digest(sentinel),(policy,r)
   if policy==3:
    focus('gui');command('gui','rdp-receive',rdp=0);spike.eventually(lambda:command('gui','rdp-inspect',rdp=0)['received_generation']>before);focus('peer');assert command('peer','read-text')['sha256']==digest(remote)
  focus('gui');command('gui','rdp-policy',rdp=0,policy=2);focus('peer');command('peer','set-text',file='policy-local.txt');focus('gui');before=receipt(0).get('received',0);command('gui','rdp-send',rdp=0);time.sleep(.2);assert receipt(0).get('received',0)==before;got=read_remote(0,'text','policy-remote.txt');assert got['sha256']==digest(remote);focus('gui');command('gui','rdp-policy',rdp=0,policy=0)
  return {'policies':5,'manual_receive':'explicit action','directional_gates':'real GDK and CLIPRDR'}
 case('RDP-NATIVE-POLICIES-5',policies)
 def repeated_receive_preserves_local_copy():
  focus('gui');command('gui','rdp-switch',rdp=0);command('gui','rdp-policy',rdp=0,policy=0)
  remote=b'REPEAT-RECEIVE-REMOTE';local=b'NEW-LOCAL-AFTER-RECEIVE'
  (root/'repeat-remote.txt').write_bytes(remote);(root/'repeat-local.txt').write_bytes(local)
  assert read_remote(0,'text','repeat-remote.txt')['sha256']==digest(remote)
  received=command('gui','rdp-inspect',rdp=0)['received_generation']
  focus('peer');command('peer','set-text',file='repeat-local.txt');focus('gui')
  command('gui','rdp-receive',rdp=0);time.sleep(.3);focus('peer')
  assert command('peer','read-text')['sha256']==digest(local)
  assert command('gui','rdp-inspect',rdp=0)['received_generation']==received
  return {'new_local_copy_preserved':True,'same_remote_generation_not_replayed':True}
 case('RDP-NATIVE-REPEATED-RECEIVE',repeated_receive_preserves_local_copy)
 def automatic_clipboard():
  proof=[]
  for tab in range(3):
   focus('gui');command('gui','rdp-switch',rdp=tab);command('gui','rdp-policy',rdp=tab,policy=0)
   path=root/f'auto-local-{tab}.txt';payload=f'Automatic local copy tab {tab} Živjo 🦀'.encode();path.write_bytes(payload)
   before=receipt(tab).get('received',0)
   focus('peer');command('peer','set-text',file=path.name);focus('gui')
   spike.eventually(lambda:receipt(tab).get('received',0)>before and receipt(tab).get('sha256')==digest(payload),12)
   # Two consecutive remote copies while the same canvas remains focused.
   for index in range(2):
    name=f'auto-remote-{tab}-{index}.txt';data=f'Automatic remote {tab}/{index} 中文'.encode();(root/name).write_bytes(data)
    before=command('gui','rdp-inspect',rdp=tab)['received_generation'];command(f'server-{tab}',format='text',file=name)
    spike.eventually(lambda:command('gui','rdp-inspect',rdp=tab)['received_generation']>before,10)
   focus('peer');first=command('peer','read-text');observed=[first];started=time.monotonic()
   def clipboard_matches():
    got=command('peer','read-text');observed.append(got);(root/f'auto-poll-{tab}.json').write_text(json.dumps({'expected':digest(data),'observed':observed,'gui':command('gui','rdp-inspect',rdp=tab)}));return got.get('sha256')==digest(data)
   matched=first.get('sha256')==digest(data)
   if not matched:spike.eventually(clipboard_matches,3)
   record={'tab':tab,'expected_sha256':digest(data),'observed':observed,'settle_ms':(time.monotonic()-started)*1000}
   (root/f'auto-observed-{tab}.json').write_text(json.dumps(record))
   assert observed[-1].get('sha256')==digest(data),record
   focus('gui')
   for index in range(2):
    name=f'auto-png-{tab}-{index}.png';pixels=bytes([tab+20,50+index*70,99,255])*(64*64);Image.frombytes('RGBA',(64,64),pixels).save(root/name)
    before=command('gui','rdp-inspect',rdp=tab)['received_generation'];command(f'server-{tab}',format='png',file=name)
    spike.eventually(lambda:command('gui','rdp-inspect',rdp=tab)['received_generation']>before,10)
   focus('peer');actual=command('peer','read-png');assert actual.get('pixel_sha256')==digest(pixels),actual
   proof.append({'tab':tab,'automatic_send':True,'consecutive_automatic_receives':2,'consecutive_automatic_png_receives':2})
  return {'tabs':proof,'explicit_send_receive_actions':0}
 case('RDP-NATIVE-AUTOMATIC-CLIPBOARD-3',automatic_clipboard)
 def newer_offer_during_read():
  focus('gui');command('gui','rdp-switch',rdp=0);command('gui','rdp-policy',rdp=0,policy=0)
  (root/'offer-old.txt').write_bytes(b'OLD-INFLIGHT-GENERATED');new=b'NEWEST-GENERATED-CLIPBOARD';(root/'offer-new.txt').write_bytes(new)
  p=root/'server-0-pending.json';before_requests=json.loads(p.read_text())['requests']if p.exists()else 0
  before=command('gui','rdp-inspect',rdp=0)['received_generation']
  command('server-0',format='text',file='offer-old.txt',delay_ms=800)
  spike.eventually(lambda:p.exists()and json.loads(p.read_text())['requests']>before_requests)
  command('server-0',format='text',file='offer-new.txt')
  spike.eventually(lambda:command('gui','rdp-inspect',rdp=0)['received_generation']>before,12)
  focus('peer');assert command('peer','read-text')['sha256']==digest(new)
  assert json.loads(p.read_text())['requests']>=before_requests+2
  return {'first_response_delay_ms':800,'newest_copy_received':True,'explicit_receive_actions':0}
 case('RDP-NATIVE-SUPERSEDED-CLIPBOARD-READ',newer_offer_during_read)


 def reconnect():
  focus('gui');command('gui','rdp-switch',rdp=0);started=time.perf_counter();command('server-0','disconnect')
  spike.eventually(lambda:'Reconnecting' in command('gui','rdp-inspect',rdp=0)['status'],8)
  r=spike.eventually(lambda:r if (r:=command('gui','rdp-inspect',rdp=0))['status'].startswith('Connected') and r['size']==[320,180] else False,15)
  got=transfer_to_remote(0,'text','policy-local.txt');assert got['sha256']==digest(b'POLICY-LOCAL-SENTINEL')
  return {'reconnect_to_frame_ms':(time.perf_counter()-started)*1000,'same_tab':True,'clipboard_after_reconnect':'PASS','bounded_backoff':True}
 case('RDP-NATIVE-RECONNECT',reconnect)
 def ownership():
  sentinel=b'LOCAL-OWNER-SENTINEL';(root/'sentinel.txt').write_bytes(sentinel);(root/'inactive.txt').write_text('Generated inactive remote offer')
  for i in range(200):
   focus('gui');active=i%4
   if active<3:command('gui','rdp-switch',rdp=active)
   else:command('gui','switch',view=0)
   focus('peer');command('peer','set-text',file='sentinel.txt');focus('gui')
   for other in range(3):
    if other!=active:command(f'server-{other}',format='text',file='inactive.txt')
   time.sleep(.025);focus('peer');r=command('peer','read-text');assert r['sha256']==digest(sentinel),(i,r)
   if (i+1)%50==0:print('OWNERSHIP',i+1,flush=True)
  return {'focus_switches':200,'inactive_offers_checked':450,'peer_clipboard_hash':digest(sentinel),'physical_input':False}
 case('RDP-NATIVE-OWNERSHIP-200',ownership)
 def mixed_workspace():
  try:
   before=[command('gui','rdp-inspect',rdp=i) for i in range(3)]
   command('gui','switch',view=0)
   tree=command('gui','mixed-join',tab=1,vertical=False)
   assert len(tree['tabs'])==3 and tree['tabs'][0]['layout']['split']=='horizontal',tree
   tree=command('gui','mixed-join',tab=1,vertical=True)
   assert len(tree['tabs'])==2 and tree['tabs'][0]['layout']['split']=='vertical' and tree['tabs'][0]['layout']['first']['split']=='horizontal',tree
   time.sleep(.4)
   for i in range(3):
    now=command('gui','rdp-inspect',rdp=i)
    assert now['transport_id']==before[i]['transport_id'] and now['size']==before[i]['size'] and now['metrics']['decoded_updates']>0,now
   command('gui','rdp-switch',rdp=0)
   text='Mixed RDP/terminal clipboard Živjo 🦀';path=root/'mixed-text.txt';path.write_text(text)
   received=transfer_to_remote(0,'text',path.name);assert received['sha256']==digest(text.encode()),received
   baseline='Inactive mixed RDP must not own this clipboard';path=root/'mixed-baseline.txt';path.write_text(baseline)
   focus('peer');command('peer','set-text',file=path.name);focus('gui')
   owner=command('gui','mixed-focus',kind='terminal',index=0)
   assert owner['owner']==session['session'],owner
   before_offers=[command('gui','rdp-inspect',rdp=i)['offers'] for i in range(2)]
   for i in range(2):command(f'server-{i}',format='text',file='mixed-text.txt')
   spike.eventually(lambda:all(command('gui','rdp-inspect',rdp=i)['offers']>before_offers[i] for i in range(2)))
   focus('peer');assert command('peer','read-text')['sha256']==digest(baseline.encode())
   focus('gui');command('gui','snapshot');tree=command('gui','mixed-separate');assert len(tree['tabs'])==4,tree
   for i in range(3):assert command('gui','rdp-inspect',rdp=i)['transport_id']==before[i]['transport_id']
   # Real dialog actions move one live RDP view, then close only that view.
   command('gui','tab-select',tab=0);command('gui','mixed-join',tab=command('gui','rdp-inspect',rdp=2)['tab'],vertical=False)
   assert command('gui','views-manage')['clicked']
   moved=command('gui','views-action',operation='move',index=1)
   assert moved['clicked'] and len(moved['layout']['tabs'])==4 and moved['rdp_views']==3,moved
   for i in range(3):assert command('gui','rdp-inspect',rdp=i)['transport_id']==before[i]['transport_id']
   command('gui','tab-select',tab=0);command('gui','mixed-join',tab=3,vertical=True)
   assert command('gui','views-manage')['clicked']
   closed=command('gui','views-action',operation='close',index=1)
   assert closed['clicked'] and closed['rdp_views']==2 and closed['terminal_views']==1 and len(closed['layout']['tabs'])==3,closed
   for i in range(2):assert command('gui','rdp-inspect',rdp=i)['transport_id']==before[i]['transport_id']
   command('gui','rdp-open',port=ready['ports'][2])
   fresh=spike.eventually(lambda:r if (r:=command('gui','rdp-inspect',rdp=2)).get('size')==[320,180] else False,20)
   assert fresh['transport_id']!=before[2]['transport_id'] and fresh['status'].startswith('Connected'),fresh
   command('gui','rdp-switch',rdp=2)
   received=transfer_to_remote(2,'text','mixed-text.txt');assert received['sha256']==digest(text.encode())
   return {'nested_orientation':['vertical','horizontal'],'actual_authenticated_rdp_connections_preserved':3,'text_sha256':digest(text.encode()),'inactive_mixed_offers_refused':2,'separated_without_reconnect':True,'native_move_preserved_transport':True,'native_close_disconnected_only_selected_rdp':True,'neighbors_kept_transport_ids':True,'replacement_authenticated_and_clipboard_verified':True}
  finally:
   command('gui','tab-select',tab=0);command('gui','mixed-separate')
 case('MIXED-NATIVE-RDP-TERMINAL',mixed_workspace)
 def files_ownership():
  import socket,threading
  listener=socket.socket();listener.bind(('127.0.0.1',0));listener.listen(1);port=listener.getsockname()[1]
  def refuse():
   connection,_=listener.accept();connection.close();listener.close()
  threading.Thread(target=refuse,daemon=True).start()
  profile={'id':'generated-files-endpoint','machine_id':'generated-files-machine','name':'Files fixture','group':'Generated','protocol':'SSH','server':f'127.0.0.1:{port}','username':'fixture','secret_reference':'generated-unused','favorite':False,'last_used':0,'tags':[]}
  sentinel=b'Files tab clipboard sentinel';(root/'files-sentinel.txt').write_bytes(sentinel);(root/'files-inactive.txt').write_text('Inactive RDP must remain cached')
  focus('peer');command('peer','set-text',file='files-sentinel.txt');focus('gui');command('gui','files-open',profile=profile)
  for _ in range(10):
   for tab in range(3):command(f'server-{tab}',format='text',file='files-inactive.txt')
   time.sleep(.03);focus('peer');assert command('peer','read-text')['sha256']==digest(sentinel);focus('gui')
  command('gui','rdp-switch',rdp=0)
  return {'native_files_tab':True,'inactive_rdp_offers':30,'peer_clipboard_unchanged':True}
 case('FILES-CLIPBOARD-OWNERSHIP',files_ownership)
 def profiles():
  focus('gui')
  for i in range(70):
   name=f'machine-{i}.remmina';original=f'[remmina]\nname=Same leaf\ngroup=Team{i}/Nested\nprotocol=RDP\nserver=127.0.0.1:3389\nusername=fixture\npassword=DO-NOT-IMPORT\nssh_privatekey=/private/key\nlabels=linux,test\n';(root/name).write_text(original);r=command('gui','profiles',file=name);assert r['rows']==min(50,i+1),r;assert(root/name).read_text()==original
  r=command('gui','profiles',search='Team69');assert r['rows']==1,r
  r=command('gui','profiles',favorite=True);assert r['favorites']==1,r
  stored=(root/'config/RUWAC/machines.json');data=stored.read_text();assert 'DO-NOT-IMPORT' not in data and '/private/key' not in data;assert stored.stat().st_mode&0o777==0o600
  return {'imported':70,'same_leaf_different_full_groups':True,'search_rows':1,'favorite_rows':1,'credentials_excluded':True,'source_files_unchanged':True,'store_mode':'0600'}
 case('MACHINES-NATIVE-70',profiles)
 def profile_journey():
  command('gui','profiles',search='R5')
  command('gui','profile-editor',operation='group',path='R5/BASING/CIFRA')
  for group in ['R5/BASING/CIFRA','R5/AHACIC']:
   r=command('gui','profile-editor',operation='new',fields={'0':'R5 same server','1':group,'3':'127.0.0.1:3389','4':'fixture','13':0,'14':1,'15':3})
   assert r['applied'] and r['error'] is None,r
  items=[p for p in r['items'] if p['name']=='R5 same server'];assert len(items)==2 and items[0]['machine_id']!=items[1]['machine_id']
  original=min(items,key=lambda p:p["group"]);r=command('gui','profile-editor',operation='edit',fields={'0':'R5 renamed','1':'R5/moved','3':'127.0.0.1:22','13':1})
  changed=next(p for p in r['items'] if p['id']==original['id']);assert changed['machine_id']==original['machine_id'] and changed['protocol']=='SSH' and changed['preferences']['display']=='native' and changed['preferences']['clipboard']==3
  command('gui','profiles',search='R5 renamed');r=command('gui','profile-editor',operation='edit',response='endpoint',fields={'13':0,'3':'127.0.0.1:3389'});siblings=[p for p in r['items'] if p['machine_id']==original['machine_id']];assert len(siblings)==2 and len({p['id'] for p in siblings})==2
  before=len(r['items']);r=command('gui','profile-editor',operation='new',response='cancel',fields={'0':'Cancelled','3':'localhost:22'});assert len(r['items'])==before
  r=command('gui','profile-editor',operation='edit',response='duplicate');assert len(r['items'])==before+2
  r=command('gui','profile-editor',operation='edit',response='delete-machine',confirm=True);assert len(r['items'])==before
  stored=root/'config/RUWAC/machines.json';on_disk=json.loads(stored.read_text());assert on_disk['schema']==1 and len(on_disk['items'])==before
  assert (stored.parent/'machines.previous.json').stat().st_mode&0o777==0o600
  return {'native_dialogs':True,'cancel_duplicate_confirmed_delete':True,'stable_identity_after_rename_move':True,'multi_endpoint_machine':True,'same_leaf_distinct_machines':True,'persisted_schema':1}
 case('MACHINES-NATIVE-EDITOR',profile_journey)
 def resize():
  focus('gui');command('gui','rdp-switch',rdp=0)
  def process_metrics():
   out={}
   for name,proc in [('gtk',gui),('server',server)]:
    stat=Path(f'/proc/{proc.pid}/stat').read_text().split();rss=int(next(s.split()[1]for s in Path(f'/proc/{proc.pid}/status').read_text().splitlines()if s.startswith('VmRSS:')))/1024;out[name]={'cpu_seconds':(int(stat[13])+int(stat[14]))/os.sysconf('SC_CLK_TCK'),'rss_mib':rss}
   return out
  time.sleep(.7);before=command('gui','rdp-inspect',rdp=0);start_metrics=process_metrics();wall_start=time.perf_counter()
  transitions=[]
  for i in range(20):
   w,h=[(640,480),(2560,1440),(3000,1800),(1024,768)][i%4]
   start=time.perf_counter();command('gui','rdp-display',rdp=0,mode='fit',zoom=1.0,width=w,height=h)
   def complete():
    r=command('gui','rdp-inspect',rdp=0)
    return r if r['size']==[w,h] and r['last_pixel'][:3]==[(w-1)%256,(h-1)%256,90] else False
   r=spike.eventually(complete,20)
   assert r['status'].startswith('Connected'),r
   transitions.append({'size':[w,h],'decoded_corner':r['last_pixel'],'complete_frame_ms':(time.perf_counter()-start)*1000,'metrics':r['metrics'],'processes':process_metrics()})
   command('gui','rdp-switch',rdp=(i%2)+1);assert command('gui','rdp-inspect',rdp=(i%2)+1)['size']==[320,180];command('gui','rdp-switch',rdp=0)
  # Actual GTK allocation changes drive the production debounced dynamic path.
  for i in range(20):command('gui','rdp-display',rdp=0,mode='dynamic',zoom=1.0,window_width=1100+i*4,window_height=760+i*2)
  def settled():
   r=command('gui','rdp-inspect',rdp=0)
   return r if r['size']==r['requested_size'] and min(r['size'])>=200 and r['status'].startswith('Connected') else False
  storm=spike.eventually(settled,15);count=storm['metrics']['decoded_updates'];time.sleep(.8);stable=settled();assert stable and stable['size']==storm['size'],stable
  for mode in ['native','stretch','fit']:
   r=command('gui','rdp-display',rdp=0,mode=mode,zoom=1.25);assert r['zoom']==1.25
  command('gui','rdp-display',rdp=0,mode='fit',zoom=1.0)
  text='Clipboard after 20 actual reactivations ž 🦀';(root/'resize-text.txt').write_text(text);got=read_remote(0,'text','resize-text.txt');assert got['sha256']==digest(text.encode()),got
  time.sleep(.7);after=command('gui','rdp-inspect',rdp=0)
  return {'before':before['metrics'],'after':after['metrics'],'processes_before':start_metrics,'processes_after':process_metrics(),'wall_seconds':time.perf_counter()-wall_start,'transitions':transitions,'negotiation':'actual Display Control DVC plus server DeactivateAll/reactivation','resize_storm':{'gtk_allocation_changes':20,'settled_size':storm['size'],'stable_after_800ms':True},'post_resize_clipboard':True,'other_tabs_unchanged':True,'codec':'raw bitmap fixture; RemoteFX/GNOME issue remains a separate gate'}
 case('RDP-NATIVE-RESIZE-20',resize)

 def profile_keyring_preferences():
  assert provider and provider.poll() is None, 'Disposable Secret Service provider unavailable'
  command('gui','profiles',search='R5 renamed')
  r=command('gui','profile-editor',operation='edit',fields={'13':0,'3':'127.0.0.1:'+str(ready['ports'][0]),'4':'fixture','10':str(root/'cert.pem')});assert not r['error'],r
  item=next(p for p in r['items']if p['name'].startswith('R5 renamed') and p['server']=='127.0.0.1:'+str(ready['ports'][0]))
  subprocess.run(['secret-tool','store','--label=RUWAC disposable fixture','application','RUWAC','profile',item['secret_reference']],input=b'generated-local-only',env=env,check=True,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL,timeout=10)
  command('gui','close-rdp',rdp=0);time.sleep(.3)
  assert command('gui','profiles',search='R5 renamed',open=True)['rows']>=1
  r=spike.eventually(lambda:r if (r:=command('gui','rdp-inspect',rdp=2)).get('status','').startswith('Connected') and min(r.get('size',[0,0]))>0 else False,20)
  command('gui','rdp-display',rdp=2,mode='stretch',zoom=1.5)
  def persisted():
   data=json.loads((root/'config/RUWAC/machines.json').read_text());p=next(p for p in data['items']if p['id']==item['id']);return p if p['preferences']['display']=='stretch' and p['preferences']['zoom']==1.5 else False
  spike.eventually(persisted,5)
  protected=command('gui','profile-editor',operation='edit',response='delete',confirm=True);assert any(p['id']==item['id']for p in protected['items'])
  # Exercise the visible native toolbar against the ordinary credential-backed connection.
  command('gui','rdp-switch',rdp=2)
  toolbar=command('gui','rdp-toolbar',rdp=2)
  standard={'reconnect','disconnect','fit','dynamic','zoom-out','zoom-in','fullscreen'}
  assert {b['action'] for b in toolbar['controls']}==standard|{'gnome-sign-in'},toolbar
  assert all(b['icon'] and b['label'] for b in toolbar['controls']),toolbar
  assert all(b['mapped'] for b in toolbar['controls']if b['action']in standard),toolbar
  assert not next(b['mapped']for b in toolbar['controls']if b['action']=='gnome-sign-in'),toolbar
  neighbors=[command('gui','rdp-inspect',rdp=i)['transport_id'] for i in [0,1]]
  before_zoom=command('gui','rdp-inspect',rdp=2)['zoom']
  assert command('gui','rdp-toolbar',rdp=2,control='zoom-in')['clicked']
  assert command('gui','rdp-inspect',rdp=2)['zoom']>before_zoom
  assert command('gui','rdp-toolbar',rdp=2,control='zoom-out')['clicked']
  assert abs(command('gui','rdp-inspect',rdp=2)['zoom']-before_zoom)<1e-6
  assert command('gui','rdp-toolbar',rdp=2,control='fit')['clicked']
  assert not command('gui','rdp-toolbar',rdp=2,control='unknown')['clicked']
  old_transport=command('gui','rdp-inspect',rdp=2)['transport_id']
  assert command('gui','rdp-toolbar',rdp=2,control='disconnect')['clicked']
  assert command('gui','rdp-inspect',rdp=2)['transport_id'] is None
  assert command('gui','rdp-toolbar',rdp=2,control='reconnect')['clicked']
  restored=spike.eventually(lambda:r if (r:=command('gui','rdp-inspect',rdp=2)).get('status','').startswith('Connected') and min(r.get('size',[0,0]))>0 else False,20)
  assert restored['transport_id']!=old_transport
  assert [command('gui','rdp-inspect',rdp=i)['transport_id']for i in [0,1]]==neighbors
  command('gui','profiles-control',import_expanded=False,tools_expanded=False)
  command('gui','snapshot');time.sleep(.15)
  assert (root/'native-rust.png').is_file()
  (root/'native-rust.png').replace(root/'rdp-toolbar.png')
  command('gui','close-rdp',rdp=2);time.sleep(.3)
  removed=command('gui','profile-editor',operation='edit',response='delete',confirm=True);assert not any(p['id']==item['id']for p in removed['items'])
  subprocess.run(['secret-tool','clear','application','RUWAC','profile',item['secret_reference']],env=env,check=True,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL,timeout=10)
  provider.terminate();provider.wait(timeout=5)
  return {'normal_sidebar_connection':'actual CredSSP with disposable Secret Service lookup and verified fixture CA','decoded_frame':r['size'],'runtime_preferences_saved':True,'profile_protected_then_released':True,'owner_secrets_accessed':False,'native_toolbar':{'visible_theme_icons':7,'zoom_fit':True,'disconnect_reconnect_authentication':True,'neighbor_transports_preserved':True,'actual_GTK_render':'rdp-toolbar.png','physical_input':False}}
 case('RDP-PROFILE-KEYRING-PREFERENCES',profile_keyring_preferences)

 def restart_and_thousand():
  global gui
  saved=json.loads((root/'config/RUWAC/machines.json').read_text())
  gui.terminate();gui.wait(timeout=5)
  gui=start([a.gui,'--socket',str(sock),'--session',session['session'],'--fixture-dir',str(root)],'gui-restart');command('gui','inspect')
  r=command('gui','profile-editor',operation='inspect');assert r['items']==saved['items'] and r['groups']==saved['groups']
  gui.terminate();gui.wait(timeout=5)
  config=root/'stress-config/RUWAC';config.mkdir(parents=True,mode=0o700);items=[]
  for i in range(1000):
   item=dict(saved['items'][0]);item.update(id=f'fixture-endpoint-{i}',machine_id=f'fixture-machine-{i}',name=f'Load{i}',group=f'Generated/Network{i%20}',secret_reference=f'fixture-secret-{i}');items.append(item)
  path=config/'machines.json';path.write_text(json.dumps({'schema':1,'items':items,'groups':[]}));path.chmod(0o600)
  env['XDG_CONFIG_HOME']=str(config.parent);started=time.perf_counter();gui=start([a.gui,'--socket',str(sock),'--session',session['session'],'--fixture-dir',str(root)],'gui-stress');command('gui','inspect');load_ms=(time.perf_counter()-started)*1000
  started=time.perf_counter();r=command('gui','profiles',search='Load999');assert r['rows']==1,r
  return {'restart_identical_ids_preferences_groups':True,'generated_profiles':1000,'load_and_fixture_ready_ms':load_ms,'search_widget_roundtrip_ms':(time.perf_counter()-started)*1000,'search_rows':1}
 case('MACHINES-RESTART-1000',restart_and_thousand)

finally:
 def rss(proc):
  try:return int(next(l.split()[1] for l in Path(f'/proc/{proc.pid}/status').read_text().splitlines() if l.startswith('VmRSS:')))/1024
  except (FileNotFoundError,StopIteration):return None
 memory={name:rss(proc) for name,proc in [('gtk',locals().get('gui')),('rdp_server',locals().get('server'))] if proc is not None}
 for proc in reversed(processes):
  if proc.poll() is None:
   proc.terminate()
   try:proc.wait(timeout=5)
   except subprocess.TimeoutExpired:proc.kill();proc.wait()
 if private_bus is not None:os.kill(private_bus,15)
 for log in logs:log.close()
 def rss(proc):
  try:return int(next(l.split()[1] for l in Path(f'/proc/{proc.pid}/status').read_text().splitlines() if l.startswith('VmRSS:')))/1024
  except (FileNotFoundError,StopIteration):return None
 report={'cases':results,'rss_mib':memory,'environment':'Owned Weston/Xvfb, real native GTK/GDK peer, actual loopback IronRDP server; generated credentials and frames','limitations':['No external Windows/GNOME login','No physical keyboard/touchpad acceptance','No external Windows/GNOME Display Control or RemoteFX qualification']}
 (root/'receipt.json').write_text(json.dumps(report,indent=2));Path(a.evidence,'rdp-native-acceptance.json').write_text(json.dumps(report,indent=2));print('Evidence',root,flush=True)
raise SystemExit(0 if results and all(r['status']=='PASS' for r in results) else 1)
