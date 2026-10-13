import hashlib,json,os,subprocess,sys,tempfile,time,re
from pathlib import Path
sys.path.insert(0,str(Path(__file__).resolve().parent));from spike import Client,eventually
from PIL import Image,ImageDraw,ImageFont
import argparse,socket,pwd,uuid
p=argparse.ArgumentParser();p.add_argument('--binary',required=True);p.add_argument('--gui',required=True);p.add_argument('--evidence',type=Path,required=True);a=p.parse_args();r=Path(__file__).resolve().parents[2];e=a.evidence;setup={'env':{k:os.environ[k]for k in ['DISPLAY','WAYLAND_DISPLAY','XDG_RUNTIME_DIR','GDK_BACKEND','RUWAC_XTEST_DISPLAY','DBUS_SESSION_BUS_ADDRESS']}};assert setup['env']['WAYLAND_DISPLAY']!='wayland-0';binary=Path(a.binary);gui=Path(a.gui);root=Path(tempfile.mkdtemp(prefix='ruwac-native-image-',dir=Path('/run/user')/str(os.getuid())));root.chmod(0o700)
for name in ['home','config','cache','data','stage']:(root/name).mkdir(mode=0o700)
env={**os.environ,**setup['env'],'XDG_CONFIG_HOME':str(root/'config'),'XDG_CACHE_HOME':str(root/'cache'),'XDG_DATA_HOME':str(root/'data'),'GSETTINGS_BACKEND':'memory','GTK_USE_PORTAL':'0'};sock=root/'control.sock';processes=[];logs=[];seq={};results=[]
def start(args,name,environment):
 log=(root/(name+'.log')).open('wb');logs.append(log);p=subprocess.Popen(args,env=environment,stdout=log,stderr=log);processes.append(p);return p
def rpc(method,params=None):
 with Client(sock)as c:return c.call(method,params)
def command(who,action,timeout=8,**kw):
 seq[who]=seq.get(who,0)+1;n=seq[who];p=root/(who+'-request.tmp');p.write_text(json.dumps({'seq':n,'action':action,**kw}));p.replace(root/(who+'-request.json'))
 def response():
  try:d=json.loads((root/(who+'-response.json')).read_text());return d['result'] if d['seq']==n else None
  except (OSError,ValueError):return None
 return eventually(response,timeout)
def focus(who):
 for _ in range(8):
  if command('gui','inspect')['active']==(who=='gui'):
   subprocess.run(['xdotool','key','Shift_L'],env={**os.environ,'DISPLAY':setup['env']['DISPLAY']},check=True,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL);time.sleep(.1)
   if who=='gui':command('gui','focus-view',view=0)
   return
  subprocess.run(['xdotool','key','--clearmodifiers','Super_L+Tab'],env={**os.environ,'DISPLAY':setup['env']['DISPLAY']},check=True);time.sleep(.2)
 raise AssertionError('owned compositor focus unavailable')
auth_root=Path(tempfile.mkdtemp(prefix='ruwac-image-auth-',dir=Path('/run/user')/str(os.getuid())));auth_root.chmod(0o700)
for name in ['host','client']:
 subprocess.run(['ssh-keygen','-q','-t','ed25519','-N','','-f',str(root/name)],check=True);(root/name).chmod(0o600)
authorized=auth_root/'authorized_keys';authorized.write_text((root/'client.pub').read_text());authorized.chmod(0o600)
with socket.socket()as probe:probe.bind(('127.0.0.1',0));port=probe.getsockname()[1]
config=root/'sshd.conf';config.write_text(f'ListenAddress 127.0.0.1\nPort {port}\nHostKey {root}/host\nPidFile {root}/sshd.pid\nAuthorizedKeysFile {authorized}\nStrictModes yes\nPasswordAuthentication no\nKbdInteractiveAuthentication no\nUsePAM no\nPermitRootLogin no\nAllowUsers {pwd.getpwuid(os.getuid()).pw_name}\nLogLevel ERROR\n')
key=(root/'host.pub').read_text().split();known=root/'known_hosts';known.write_text(f'[127.0.0.1]:{port} {key[0]} {key[1]}\n');known.chmod(0o600)
profile={'id':'endpoint_'+uuid.uuid4().hex,'machine_id':'machine_'+uuid.uuid4().hex,'name':'Owned remote Codex','group':'Fixture','protocol':'SSH','server':f'127.0.0.1:{port}','username':pwd.getpwuid(os.getuid()).pw_name,'secret_reference':'generated-unused','favorite':False,'last_used':0,'tags':[],'preferences':{'identity_file':str(root/'client'),'known_hosts':str(known)}}
(root/'config/RUWAC').mkdir(mode=0o700);library=root/'config/RUWAC/machines.json';library.write_text(json.dumps({'schema':1,'items':[profile]}));library.chmod(0o600)
remote_stage=root/'remote-stage';remote_stage.mkdir(mode=0o700)
try:
 server=start(['/usr/sbin/sshd','-D','-e','-f',str(config)],'sshd',os.environ.copy());time.sleep(.4);assert server.poll()is None
 daemon=start([str(binary),'--socket',str(sock),'serve'],'daemon',os.environ.copy());eventually(sock.exists)
 session=rpc('session.create',{'name':'Owned real Codex consumer','argv':['codex','--no-alt-screen','--sandbox','read-only','-c','notify=[]','-c',f'projects."{root}".trust_level="trusted"','-C',str(root)]});pane=session['pane']
 other=rpc('session.create',{'name':'Owned ordinary terminal','argv':['python3','-c','import time;print("ORDINARY_TERMINAL",flush=True);time.sleep(120)']})
 window=start([str(gui),'--socket',str(sock),'--session',session['session'],'--session',other['session'],'--fixture-dir',str(root),'--staging-dir',str(root/'stage')],'gui',env)
 peer=start(['python3',str(r/'tests/fixtures/desktop_clipboard.py'),str(root)],'peer',env);eventually(lambda:(root/'peer-ready.json').exists());command('gui','tab-select',tab=0);eventually(lambda:command('gui','inspect')['drawn'][0]>0,20)
 def ready():
  text=rpc('pane.capture',{'pane':pane,'lines':100})['text'];(root/'consumer-screen.txt').write_text(text);return 'Ask Codex' in text and ('GPT-'in text or 'context left'in text)
 eventually(ready,30)
 image=Image.new('RGB',(640,240),'#16365a');ImageDraw.Draw(image).text((70,70),'729481',font=ImageFont.load_default(size=72),fill='white');image.save(root/'generated.png');(root/'generated.png').chmod(0o600);hash=hashlib.sha256((root/'generated.png').read_bytes()).hexdigest()
 command('gui','tab-select',tab=0);focus('peer');command('peer','set-png',file='generated.png');focus('gui');command('gui','focus-view',view=0)
 assert command('gui','remote-image-dialog',operation='show')['clicked']
 refusal=command('gui','remote-image-dialog',operation='send',socket=str(sock),pane=pane,directory=str(remote_stage),confirm=False)
 assert 'Confirm' in refusal['status'] and '[Image #1]'not in rpc('pane.capture',{'pane':pane,'lines':100})['text']
 results.append({'id':'REMOTE_NATIVE_REQUIRES_EXPLICIT_CONFIRMATION','PASS':True})
 refusal=command('gui','remote-image-dialog',operation='send',socket='../bad',confirm=True)
 assert 'Absolute' in refusal['status']
 results.append({'id':'REMOTE_NATIVE_REFUSES_INVALID_TARGET','PASS':True})
 profile['name']='Updated owned remote Codex';temporary=library.with_suffix('.new');temporary.write_text(json.dumps({'schema':1,'items':[profile]}));temporary.chmod(0o600);temporary.replace(library);time.sleep(2)
 stale=command('gui','remote-image-dialog',operation='send',socket=str(sock),pane=pane,directory=str(remote_stage),confirm=True)
 assert 'changed' in stale['status'] and '[Image #1]'not in rpc('pane.capture',{'pane':pane,'lines':100})['text']
 results.append({'id':'REMOTE_NATIVE_PROFILE_CHANGE_REQUIRES_REVIEW','PASS':True})
 attachment_started=time.monotonic();command('gui','remote-image-dialog',operation='send',socket=str(sock),pane=pane,directory=str(remote_stage),confirm=True)
 eventually(lambda:'PNG attached to the verified remote' in command('gui','inspect')['status'],25)
 assert '[Image #1]' in rpc('pane.capture',{'pane':pane,'lines':100})['text']
 staged=list(remote_stage.glob('*.png'));assert len(staged)==1
 file=staged[0];assert file.stat().st_mode&0o777==0o600 and hashlib.sha256(file.read_bytes()).hexdigest()==hash
 results.append({'id':'NATIVE_GDK_AUTHENTICATED_SSH_REAL_CODEX_DRAFT','PASS':True,'sha256':hash,'attachment_ms':round((time.monotonic()-attachment_started)*1000,2),'actual_button':True,'actual_GDK_Wayland':True,'generated_known_host_and_private_key':True,'consumer':'real Codex over loopback SSH','production_submits':False})
 # Cancel a slow PNG read by switching the real owned Wayland focus.
 noisy=Image.frombytes('RGB',(512,512),os.urandom(512*512*3));noisy.save(root/'cancel.png');focus('peer');command('peer','set-png',file='cancel.png');focus('gui')
 before=rpc('pane.capture',{'pane':pane,'lines':100})['text'];seq['gui']+=1;n=seq['gui'];temp=root/'gui-request.tmp';temp.write_text(json.dumps({'seq':n,'action':'attach-image','delay_ms':100}));temp.replace(root/'gui-request.json');time.sleep(.05)
 subprocess.run(['xdotool','key','--clearmodifiers','Super_L+Tab'],env={**os.environ,'DISPLAY':setup['env']['DISPLAY']},check=True)
 def cancelled():
  try:d=json.loads((root/'gui-response.json').read_text());return d['result'] if d['seq']==n else None
  except(OSError,ValueError):return None
 cancellation=eventually(cancelled,8);assert 'cancel'in cancellation.get('error','').lower(),cancellation;assert rpc('pane.capture',{'pane':pane,'lines':100})['text']==before
 results.append({'id':'WAYLAND_FOCUS_CANCEL','PASS':True,'result':cancellation})
 focus('gui');command('gui','tab-select',tab=1);command('gui','focus-view',view=1);before=rpc('pane.capture',{'pane':other['pane'],'lines':100})['text'];refusal=command('gui','attach-image',view=1,timeout=12);assert 'error'in refusal,refusal;assert rpc('pane.capture',{'pane':other['pane'],'lines':100})['text']==before
 results.append({'id':'ORDINARY_TERMINAL_REFUSED','PASS':True,'result':refusal})
 # Close the frontend before submitting: the persistent Codex draft must still read its file.
 command('gui','workspace-close');window.wait(timeout=10);assert file.exists(),'draft attachment removed on frontend close';assert rpc('job.query',{'pane':pane})['pane_ended']==False
 results.append({'id':'CLOSE_PRESERVES_PENDING_DRAFT_FILE_AND_PTY','PASS':True})
 rpc('pane.sendText',{'pane':pane,'text':'Read the six digits in the attached image. Reply with only those digits. Do not use tools.'});time.sleep(.8);rpc('pane.sendKey',{'pane':pane,'key':'Enter'})
 eventually(lambda:re.search(r'•\s*729481',rpc('pane.capture',{'pane':pane,'lines':100})['text']),60)
 results.append({'id':'ACTUAL_CONSUMPTION_AFTER_FRONTEND_CLOSE','PASS':True})
 result={'binary':subprocess.check_output([str(binary),'--version'],text=True).strip(),'gui_sha256':hashlib.sha256(gui.read_bytes()).hexdigest(),'results':results,'root':str(root),'physical_desktop':'not tested','remote_SSH_image':'real authenticated loopback SSH transport; external host consumption not tested'};(e/'native-remote-receipt.json').write_text(json.dumps(result,indent=2)+'\n');print('NATIVE_IMAGE_ACCEPTANCE PASS',result['binary'],flush=True)
finally:
 for p in reversed(processes):
  if p.poll()is None:p.terminate()
  try:p.wait(timeout=10)
  except subprocess.TimeoutExpired:p.kill();p.wait()
 for log in logs:log.close()
 authorized.unlink(missing_ok=True);auth_root.rmdir()
