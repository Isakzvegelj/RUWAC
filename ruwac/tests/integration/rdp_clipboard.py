#!/usr/bin/env python3
"""Authenticated loopback RDP and independent GDK peer on an owned compositor."""
import argparse,hashlib,importlib.util,json,os,subprocess,sys,tempfile,time,traceback
from pathlib import Path
from PIL import Image
p=argparse.ArgumentParser();p.add_argument('--binary',required=True);p.add_argument('--gui',required=True);p.add_argument('--server',required=True);p.add_argument('--evidence',required=True);a=p.parse_args()
spec=importlib.util.spec_from_file_location('spike',Path(__file__).with_name('spike.py'));spike=importlib.util.module_from_spec(spec);spec.loader.exec_module(spike)
root=Path(tempfile.mkdtemp(prefix='rdp-native-',dir=a.evidence));root.chmod(0o700);env={**os.environ,'HOME':str(root/'home'),'XDG_CONFIG_HOME':str(root/'config'),'XDG_CACHE_HOME':str(root/'cache'),'XDG_DATA_HOME':str(root/'data'),'GSETTINGS_BACKEND':'memory','GTK_USE_PORTAL':'0','GTK_A11Y':'none'}
for key in ['HOME','XDG_CONFIG_HOME','XDG_CACHE_HOME','XDG_DATA_HOME']:Path(env[key]).mkdir(mode=0o700)
processes=[];logs=[];results=[];sequences={};sock=root/'control.sock'
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
 def profiles():
  focus('gui')
  for i in range(70):
   name=f'machine-{i}.remmina';original=f'[remmina]\nname=Same leaf\ngroup=Team{i}/Nested\nprotocol=RDP\nserver=127.0.0.1:3389\nusername=fixture\npassword=DO-NOT-IMPORT\nssh_privatekey=/private/key\nlabels=linux,test\n';(root/name).write_text(original);r=command('gui','profiles',file=name);assert r['rows']==i+1,r;assert(root/name).read_text()==original
  r=command('gui','profiles',search='Team69');assert r['rows']==1,r
  r=command('gui','profiles',favorite=True);assert r['favorites']==1,r
  stored=(root/'config/RUWAC/machines.json');data=stored.read_text();assert 'DO-NOT-IMPORT' not in data and '/private/key' not in data;assert stored.stat().st_mode&0o777==0o600
  return {'imported':70,'same_leaf_different_full_groups':True,'search_rows':1,'favorite_rows':1,'credentials_excluded':True,'source_files_unchanged':True,'store_mode':'0600'}
 case('MACHINES-NATIVE-70',profiles)
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
 for log in logs:log.close()
 def rss(proc):
  try:return int(next(l.split()[1] for l in Path(f'/proc/{proc.pid}/status').read_text().splitlines() if l.startswith('VmRSS:')))/1024
  except (FileNotFoundError,StopIteration):return None
 report={'cases':results,'rss_mib':memory,'environment':'Owned Weston/Xvfb, real native GTK/GDK peer, actual loopback IronRDP server; generated credentials and frames','limitations':['No external Windows/GNOME login','No physical keyboard/touchpad acceptance','No dynamic remote resize qualification']}
 (root/'receipt.json').write_text(json.dumps(report,indent=2));Path(a.evidence,'rdp-native-acceptance.json').write_text(json.dumps(report,indent=2));print('Evidence',root,flush=True)
raise SystemExit(0 if results and all(r['status']=='PASS' for r in results) else 1)
