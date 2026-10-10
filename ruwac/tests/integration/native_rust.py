#!/usr/bin/env python3
"""Rust GTK acceptance on a test compositor; synthetic input is explicitly labelled."""
import argparse,hashlib,importlib.util,json,os,subprocess,sys,tempfile,time,traceback,struct,zlib
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('--binary',required=True);p.add_argument('--gui',required=True);p.add_argument('--evidence',required=True);p.add_argument('--no-clipboard',action='store_true');p.add_argument('--primary-unavailable',action='store_true');a=p.parse_args()
spec=importlib.util.spec_from_file_location('spike',Path(__file__).with_name('spike.py'));spike=importlib.util.module_from_spec(spec);spec.loader.exec_module(spike)
root=Path(tempfile.mkdtemp(prefix='native-rust-',dir=a.evidence));root.chmod(0o700);
fixture_env={**os.environ,'HOME':str(root/'home'),'XDG_CONFIG_HOME':str(root/'config'),'XDG_CACHE_HOME':str(root/'cache'),'XDG_DATA_HOME':str(root/'data'),'GSETTINGS_BACKEND':'memory','GTK_USE_PORTAL':'0','GTK_A11Y':'none'}
for key in ['HOME','XDG_CONFIG_HOME','XDG_CACHE_HOME','XDG_DATA_HOME']:Path(fixture_env[key]).mkdir(mode=0o700)
stage=root/'stage';stage.mkdir(mode=0o700);sock=root/'control.sock';binary=Path(a.binary).resolve();gui=Path(a.gui).resolve();receiver=Path(__file__).parents[1]/'fixtures/receiver.py';peer_script=receiver.with_name('desktop_clipboard.py');results=[];processes=[];logs=[];sequences={}
def start(argv,name):
 log=(root/(name+'.log')).open('wb');logs.append(log);proc=subprocess.Popen(argv,stdout=log,stderr=log,env=fixture_env);processes.append(proc);return proc
def rpc(m,p=None):
 with spike.Client(sock) as c:return c.call(m,p)
def command(who,action,timeout=6,**kw):
 seq=sequences.get(who,0)+1;sequences[who]=seq;temp=root/(who+'-request.tmp');temp.write_text(json.dumps({'seq':seq,'action':action,**kw}));temp.replace(root/(who+'-request.json'))
 def read():
  try:r=json.loads((root/(who+'-response.json')).read_text());return r['result'] if r['seq']==seq else None
  except (FileNotFoundError,ValueError):return None
 return spike.eventually(read,timeout)
def focus(who):
 if os.environ.get('RUWAC_XTEST_DISPLAY'):
  for _ in range(4):
   state=command('gui','inspect').get('active')
   if state==(who=='gui'):
    subprocess.run(['xdotool','key','Shift_L'],env={**os.environ,'DISPLAY':os.environ['RUWAC_XTEST_DISPLAY']},check=True,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL);time.sleep(.1);return
   subprocess.run(['xdotool','key','--clearmodifiers','Super_L+Tab'],env={**os.environ,'DISPLAY':os.environ['RUWAC_XTEST_DISPLAY']},check=True)
   time.sleep(.15)
  raise AssertionError('isolated compositor did not focus '+who)
def case(name,fn):
 try:detail=fn();results.append({'id':name,'status':'PASS','details':detail});print(name,'PASS',flush=True)
 except Exception as e:results.append({'id':name,'status':'FAIL','error':str(e),'traceback':traceback.format_exc()});print(name,'FAIL',str(e),flush=True)
def png(path,width,height):
 def chunk(kind,data):return struct.pack('!I',len(data))+kind+data+struct.pack('!I',zlib.crc32(kind+data))
 raw=b''.join(b'\0'+os.urandom(width*4) for _ in range(height));path.write_bytes(b'\x89PNG\r\n\x1a\n'+chunk(b'IHDR',struct.pack('!IIBBBBB',width,height,8,6,0,0,0))+chunk(b'IDAT',zlib.compress(raw,1))+chunk(b'IEND',b''))
server=start([str(binary),'--socket',str(sock),'serve'],'daemon');native=peer=None
try:
 spike.eventually(lambda:sock.exists())
 def create(name,file):return rpc('session.create',{'name':name,'argv':[sys.executable,str(receiver),str(root/file)]})
 one=create('Rust native one','one.bin');two=create('Rust native two','two.bin');split=rpc('pane.create',{'window':one['window'],'argv':[sys.executable,str(receiver),str(root/'split.bin')]});targets=[one['pane'],split['pane'],two['pane']];files=[root/'one.bin',root/'split.bin',root/'two.bin'];pids=[one['pid'],split['pid'],two['pid']]
 for pane in targets:rpc('pane.wait',{'pane':pane,'contains':'READY','timeout_ms':5000})
 native=start([str(gui),'--socket',str(sock),'--session',one['session'],'--session',two['session'],'--fixture-dir',str(root),'--staging-dir',str(stage)],'gui')
 peer=start([sys.executable,str(peer_script),str(root)],'peer');spike.eventually(lambda:(root/'peer-ready.json').exists())
 def ready():
  r=command('gui','inspect');return r if r.get('panes')==[2,1] else False
 initial=spike.eventually(ready,15)
 def native_tabs():
  command('gui','switch',view=1,pane=targets[2]);time.sleep(.2);command('gui','switch',view=0,pane=targets[0]);time.sleep(.2);r=command('gui','inspect');assert r['display']=='GdkWaylandDisplay' and all(n>0 for n in r['drawn']),r;command('gui','snapshot');assert (root/'native-rust.png').stat().st_size>1000;return r
 case('GUI-R1',native_tabs)
 if not a.no_clipboard:
  def grid_selection():
   focus('gui');command('gui','switch',view=0,pane=targets[0]);command('gui','grid-copy');time.sleep(.1);focus('peer');r=command('peer','read-text');assert r.get('sha256')==hashlib.sha256(b'RUWAC_RECEIVER_READY ').hexdigest() or r.get('sha256')==hashlib.sha256(b'RUWAC_RECEIVER_READY').hexdigest(),r;return {'canonical_grid_selection':True,'copy':r,'selection_events':'synthetic range; physical drag untested'}
  case('CLIP-canonical-selection',grid_selection)
 def switches():
  expected=[b'',b'',b'']
  for i in range(200):
   j=i%3;text=f'SW{i:04}_';expected[j]+=text.encode();command('gui','switch',view=0 if j<2 else 1,pane=targets[j],text=text)
  for file,expected_bytes in zip(files,expected):spike.eventually(lambda f=file,e=expected_bytes:f.read_bytes()==e,10)
  command('gui','switch',view=0,pane=targets[0]);return {'switches':200,'targeted_exact':True,'input':'synthetic handler and tab changes; not physical keyboard','sha256':[hashlib.sha256(f.read_bytes()).hexdigest() for f in files]}
 case('GUI-R2',switches)
 def screen_keys():
  command('gui','switch',view=0,pane=targets[0]);before=files[0].read_bytes()
  def prefix(key):command('gui','key',key='a',mods=4);command('gui','key',key=key)
  command('gui','key',key='a',mods=4);command('gui','key',key='a',mods=4);spike.eventually(lambda:files[0].read_bytes()==before+b'\x01')
  split_before=files[1].read_bytes();prefix('Tab');command('gui','key',key='F');spike.eventually(lambda:files[1].read_bytes()==split_before+b'F');prefix('Tab')
  def active():return next(x['active_window'] for x in rpc('session.list') if x['id']==one['session'])
  prefix('c');new_window=spike.eventually(lambda:active() if active()!=one['window'] else False);time.sleep(.2)
  prefix('n');spike.eventually(lambda:active()==one['window']);time.sleep(.2)
  prefix('p');spike.eventually(lambda:active()==new_window);time.sleep(.2)
  prefix('0');spike.eventually(lambda:active()==one['window']);time.sleep(.2);command('gui','switch',view=0,pane=targets[0])
  return {'native_callback_cards':['Ctrl+A Ctrl+A','Ctrl+A Tab','Ctrl+A c','Ctrl+A n','Ctrl+A p','Ctrl+A 0'],'window_creation_and_cycles':'PASS','nested_control_byte':'exact','copy_search_and_physical_keys':'PARTIAL / UNTESTED'}
 case('GUI-R5-Screen',screen_keys)
 def mouse():
  before=files[0].read_bytes();command('gui','mouse',view=0,x=15.0,y=15.0,button=0);time.sleep(.1);assert files[0].read_bytes()==before
  rpc('pane.sendText',{'pane':targets[0],'text':'/mouse-on/'});time.sleep(.3);command('gui','mouse',view=0,x=15.0,y=15.0,button=0);spike.eventually(lambda:files[0].read_bytes().endswith(b'\x1b[<0;2;1M'))
  rpc('pane.sendText',{'pane':targets[0],'text':'/mouse-off/'});time.sleep(.3);before=files[0].read_bytes();command('gui','mouse',view=0,x=15.0,y=15.0,button=0);time.sleep(.1);assert files[0].read_bytes()==before;return {'enabled_sgr':'PASS','disabled_leak':'none','physical_touchpad':'BLOCKED'}
 case('INPUT-R1',mouse)
 if not a.no_clipboard:
  selection="Native selection žółć 🧪\nline\t'quotes'".encode()
  def copied(primary=False):
   focus('gui');command('gui','copy',primary=primary);time.sleep(.1);focus('peer');r=command('peer','read-text',primary=primary);assert r.get('sha256')==hashlib.sha256(selection).hexdigest(),r;return r
  case('CLIP-R1-copy',copied)
  if a.primary_unavailable:results.append({'id':'CLIP-R2-primary','status':'BLOCKED','reason':'Test compositor does not advertise a primary-selection protocol; failed diagnostic retained'})
  else:case('CLIP-R2-primary',lambda:copied(True))
  for size in [10*1024,1024*1024]:
   def transfer(size=size):
    unit='žółć 🧪\n\t"quotes"\\'.encode();payload=(unit*(size//len(unit)))+b'x'*(size%len(unit));(root/'text-fixture.utf8').write_bytes(payload);focus('peer');command('peer','set-text',file='text-fixture.utf8');time.sleep(.1);focus('gui');r=command('gui','read-text',timeout=10);assert r.get('sha256')==hashlib.sha256(payload).hexdigest(),r
    before=files[0].read_bytes();command('gui','paste');spike.eventually(lambda:files[0].read_bytes()==before+payload,20);return {'bytes':size,'sha256':r['sha256'],'exact_paste':True,'responsive':command('gui','inspect')}
   case('CLIP-R3-'+str(size),transfer)
  def independent():
   rpc('buffer.set',{'session':one['session'],'text':'INTERNAL_A'});(root/'text-fixture.utf8').write_text('EXTERNAL_B');focus('peer');command('peer','set-text',file='text-fixture.utf8');time.sleep(.1);focus('gui');command('gui','read-text');assert rpc('buffer.get',{'session':one['session']})['text']=='INTERNAL_A';return {'internal_unchanged':True}
  case('CLIP-R2-internal',independent)
  for width,height in [(64,64),(2560,1440)]:
   def image(width=width,height=height):
    path=root/f'fixture-{width}.png';png(path,width,height);focus('peer');command('peer','set-png',file=path.name);time.sleep(.1);focus('gui');r=command('gui','read-image',timeout=20);assert r.get('sha256')==hashlib.sha256(path.read_bytes()).hexdigest(),r;staged=Path(r['path']);assert staged.stat().st_mode&0o777==0o600;assert (r['width'],r['height'])==(width,height);assert 'image/png' not in rpc('schema').get('methods',[]);return r
   case('IMAGE-R1-'+str(width),image)
  def cap():
   r=command('gui','read-image',limit=100);assert 'cap' in r.get('error',''),r;return r
  case('IMAGE-cap',cap)
 if not a.no_clipboard:
  def cancellation(action):
   focus('peer')
   if action=='read-text':
    (root/'cancel-text.utf8').write_bytes(b'X'*(1024*1024));command('peer','set-text',file='cancel-text.utf8')
   else:command('peer','set-png',file='fixture-2560.png')
   time.sleep(.1);focus('gui');seq=sequences['gui']+1;sequences['gui']=seq
   temp=root/'gui-request.tmp';temp.write_text(json.dumps({'seq':seq,'action':action,'delay_ms':100}));temp.replace(root/'gui-request.json');time.sleep(.05)
   command('gui','cancel')
   def cancelled():
    r=json.loads((root/'gui-response.json').read_text());return r['result'] if r['seq']==seq else None
   result=spike.eventually(cancelled,5);assert 'cancel' in result.get('error','').lower(),result
   assert not list(stage.glob('transfer-*')), 'partial staging file leaked';return result
  case('CLIP-stale-cancel',lambda:cancellation('read-text'))
  case('IMAGE-cancel-cleanup',lambda:cancellation('read-image'))
 if not a.no_clipboard:
  def disabled():
   command('gui','policy',disabled=True);r=command('gui','read-text');assert 'error' in r,r;command('gui','policy',disabled=False);return {'disabled_transport_rejected':True}
  case('CLIP-disabled-policy',disabled)
 def queue():
  before=command('gui','inspect');command('gui','delay-queue');mid=command('gui','inspect');assert 'Queue' in mid['status'],mid;assert mid['ticks']>before['ticks'];time.sleep(1.3);after=command('gui','inspect');assert after['ticks']>mid['ticks']+10;return {'before':before,'during':mid,'after':after}
 case('GUI-R4-delayed-IPC',queue)
 def detach():
  command('gui','quit');native.wait(timeout=5)
  for pid in pids:os.kill(pid,0)
  with spike.Client(sock) as c:
   c.call('session.attach',{'session':one['session'],'rows':30,'cols':90});c.call('client.resize',{'rows':34,'cols':94});frame=c.call('client.render');assert all(p['grid']['cols']==94 for p in frame['panes']);before=files[0].read_bytes();c.call('client.input',{'pane':targets[0],'kind':'keyboard','text':'AFTER_NATIVE'});spike.eventually(lambda:files[0].read_bytes()==before+b'AFTER_NATIVE');c.call('session.detach')
  tui=spike.Tui(binary,sock,one['session'],root/'reattach.pty');spike.eventually(lambda:b'\x1b[H\x1b[2J' in tui.data);tui.finish();return {'same_pids':pids,'tui_reattach':'PASS','resize':'PASS'}
 case('GUI-R3',detach)
 assert not list(stage.iterdir()),'completed staging files not cleaned on GUI quit'
finally:
 if peer and peer.poll()is None:
  try:command('peer','quit')
  except Exception:pass
 for proc in reversed(processes):
  if proc.poll()is None:proc.terminate()
  try:proc.wait(timeout=8)
  except subprocess.TimeoutExpired:proc.kill();proc.wait()
 for log in logs:log.close()
 (root/'acceptance.json').write_text(json.dumps({'results':results,'binary_sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),'gui_sha256':hashlib.sha256(gui.read_bytes()).hexdigest(),'physical_desktop':'BLOCKED','real_rdp':'BLOCKED'},indent=2)+'\n')
 print('NATIVE_RUST_EVIDENCE',root,flush=True)
if any(r['status']=='FAIL' for r in results):raise SystemExit(1)
