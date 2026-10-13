#!/usr/bin/env python3
"""Generated-payload clipboard acceptance against an explicitly authorized RDP/SSH fixture.

The caller creates a private authenticated GUI fixture and an independent remote
GDK peer with an empty clipboard. This test never creates credentials or logs
clipboard payloads; local input is synthetic on an isolated compositor.
"""
import json,subprocess,shlex,time,os,hashlib,tempfile
from pathlib import Path
from PIL import Image
import argparse,re
p=argparse.ArgumentParser()
for name in ['fixture','display-env','ssh-host','remote-peer-root','evidence']:p.add_argument('--'+name,required=True)
p.add_argument('--width',type=int,default=64);p.add_argument('--height',type=int,default=64)
p.add_argument('--bidirectional',action='store_true');p.add_argument('--authorized-external',action='store_true')
a=p.parse_args()
if not a.authorized_external:p.error('Explicit external-host authorization is required')
if not re.fullmatch(r'[A-Za-z0-9_][A-Za-z0-9_.@-]{0,254}',a.ssh_host):p.error('Invalid SSH host alias')
remote_path=Path(a.remote_peer_root)
if not remote_path.is_absolute()or '..'in remote_path.parts:p.error('An absolute owned remote peer directory is required')
if not(1<=a.width<=4096 and 1<=a.height<=4096):p.error('Image dimensions exceed fixture limits')
r=Path(__file__).resolve().parents[2];e=Path(a.evidence);e.mkdir(mode=0o700,parents=True,exist_ok=True)
if e.is_symlink()or e.stat().st_uid!=os.getuid()or e.stat().st_mode&0o077:p.error('Evidence must be an owned private directory')
d=json.loads(Path(a.fixture).read_text());root=Path(d['root']);remote=str(remote_path)
display=json.loads(Path(a.display_env).read_text())['env']
if not display.get('WAYLAND_DISPLAY')or display['WAYLAND_DISPLAY']=='wayland-0':p.error('An isolated compositor is required')
env={**os.environ,**display,'GTK_USE_PORTAL':'0','GSETTINGS_BACKEND':'memory'}
ssh=['ssh','-T','-o','BatchMode=yes','-o','StrictHostKeyChecking=yes','-o','ForwardAgent=no','-o','ClearAllForwardings=yes',a.ssh_host];seq={};cases=[]
def remote_python(code):return subprocess.check_output(ssh+['python3 -c '+shlex.quote(code)],text=True,timeout=15)
def command(who,action,**kw):
 if who=='remote':
  seq[who]=seq.get(who,0)+1;n=seq[who];body=json.dumps({'seq':n,'action':action,**kw});remote_python('from pathlib import Path;p=Path('+repr(remote)+');t=p/"peer-request.tmp";t.write_text('+repr(body)+');t.replace(p/"peer-request.json")')
 else:
  path=root if who=='gui'else peer_root;base='gui'if who=='gui'else'peer';current=json.loads((path/(base+'-response.json')).read_text())['seq']if(path/(base+'-response.json')).exists()else 0;n=max(seq.get(who,0),current)+1;seq[who]=n;t=path/'request.tmp';t.write_text(json.dumps({'seq':n,'action':action,**kw}));t.replace(path/(base+'-request.json'))
 end=time.monotonic()+15
 while time.monotonic()<end:
  try:v=json.loads(remote_python('from pathlib import Path;print(Path('+repr(remote+'/peer-response.json')+').read_text())'))if who=='remote'else json.loads((path/(base+'-response.json')).read_text())
  except(Exception):time.sleep(.05);continue
  if v['seq']==n:return v['result']
  time.sleep(.05)
 raise AssertionError('Peer response timeout '+who)
def focus(who):
 for _ in range(8):
  if command('gui','inspect')['active']==(who=='gui') and (who=='gui' or command('peer','inspect')['window_active']):
   subprocess.run(['xdotool','key','Shift_L'],env=env,check=True,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
   if who=='gui':command('gui','rdp-switch',rdp=0)
   return
  subprocess.run(['xdotool','key','--clearmodifiers','Super_L+Tab'],env=env,check=True);time.sleep(.2)
 raise AssertionError('Owned display focus unavailable')
def wait(test,seconds=15):
 end=time.monotonic()+seconds
 while time.monotonic()<end:
  if value:=test():return value
  time.sleep(.1)
 raise AssertionError('Transfer condition timeout')
peer_root=Path(tempfile.mkdtemp(prefix='local-peer-',dir=e));peer_root.chmod(0o700)
ready=json.loads(remote_python('from pathlib import Path;print(Path('+repr(remote+'/peer-ready.json')+').read_text())'));seq['remote']=40000
empty=command('remote','read-text');assert empty.get('bytes')==0, 'Remote clipboard is not empty; no data overwritten'
peer=subprocess.Popen(['python3',str(r/'tests/fixtures/desktop_clipboard.py'),str(peer_root)],env=env,stdout=(e/'local-peer.log').open('wb'),stderr=subprocess.STDOUT)
try:
 wait(lambda:(peer_root/'peer-ready.json').exists());info=command('gui','rdp-inspect',rdp=0);assert info['gnome_auto']['step']=='Complete'and info['gnome_auto']['handovers']>=2
 payload='RUWAC_FROM_EVF_Ž_中文_🦀\nRemote generated line\n'.encode();local_payload='RUWAC_FROM_CVF_Ž_中文_🦀\nLocal generated line\n'.encode();(peer_root/'remote-generated.txt').write_bytes(payload);(peer_root/'local-generated.txt').write_bytes(local_payload)
 size=(a.width,a.height);image=Image.new('RGBA',size,(31,117,213,255));image.save(peer_root/'remote-generated.png');pixel=hashlib.sha256(image.tobytes()).hexdigest();local_image=Image.new('RGBA',size,(211,73,19,255));local_image.save(peer_root/'local-generated.png');local_pixel=hashlib.sha256(local_image.tobytes()).hexdigest()
 subprocess.run([os.environ.get('RUWAC_COPY_BRIDGE','rsync'),'-ac','-e','ssh -o BatchMode=yes -o StrictHostKeyChecking=yes -o ForwardAgent=no -o ClearAllForwardings=yes',str(peer_root/'remote-generated.txt'),str(peer_root/'local-generated.txt'),str(peer_root/'remote-generated.png'),str(peer_root/'local-generated.png'),a.ssh_host+':'+remote+'/'],env={**os.environ,'ORIG_RSYN_EXECUTABLE':'/usr/bin/rsync'},check=True)
 # Establish receive-only policy before copying fresh remote data; wait for its offer.
 focus('gui');command('gui','rdp-policy',rdp=0,policy=(0 if a.bidirectional else 2));time.sleep(.4)
 info=command('gui','rdp-inspect',rdp=0);before=info['received_generation'];offers=info['offers']
 command('remote','set-text',file='remote-generated.txt')
 wait(lambda:command('gui','rdp-inspect',rdp=0)['offers']>offers)
 time.sleep(.2);command('gui','rdp-receive',rdp=0)
 wait(lambda:command('gui','rdp-inspect',rdp=0)['received_generation']>before);time.sleep(.2);focus('peer');actual=command('peer','read-text');assert actual.get('sha256')==hashlib.sha256(payload).hexdigest(),actual
 cases.append({'id':'REAL_FEDORA_RDP_REMOTE_TO_LOCAL_UNICODE_TEXT','PASS':True})
 focus('peer');command('peer','set-text',file='local-generated.txt');assert command('peer','read-text')['sha256']==hashlib.sha256(local_payload).hexdigest();time.sleep(.3);focus('gui');time.sleep(.3);command('gui','rdp-policy',rdp=0,policy=(0 if a.bidirectional else 1));command('gui','rdp-send',rdp=0)
 def remote_text_matches():
  value=command('remote','read-text');(e/'text-response.json').write_text(json.dumps(value));return value if value.get('lf_sha256')==hashlib.sha256(local_payload).hexdigest()else None
 actual=wait(remote_text_matches);cases.append({'id':'REAL_FEDORA_RDP_LOCAL_TO_REMOTE_UNICODE_TEXT','PASS':True,'response':actual,'newline_comparison':'Canonical LF; wire/server CRLF retained as reported'})
 focus('gui');command('gui','rdp-policy',rdp=0,policy=(0 if a.bidirectional else 2));before=command('gui','rdp-inspect',rdp=0)['received_generation'];command('remote','set-png',file='remote-generated.png');command('gui','rdp-receive',rdp=0)
 wait(lambda:command('gui','rdp-inspect',rdp=0)['received_generation']>before);focus('peer');actual=command('peer','read-png');assert actual.get('pixel_sha256')==pixel,actual;cases.append({'id':'REAL_FEDORA_RDP_REMOTE_TO_LOCAL_PNG','PASS':True,'dimensions':size,'pixel_sha256':pixel})
 focus('peer');command('peer','set-png',file='local-generated.png');assert command('peer','read-png')['pixel_sha256']==local_pixel;time.sleep(.3);focus('gui');time.sleep(.3);command('gui','rdp-policy',rdp=0,policy=(0 if a.bidirectional else 1));command('gui','rdp-send',rdp=0)
 def remote_png_matches():
  value=command('remote','read-png');(e/'png-response.json').write_text(json.dumps({'read':value,'peer':command('remote','inspect'),'gui':command('gui','rdp-inspect',rdp=0)}));return value if value.get('pixel_sha256')==local_pixel else None
 actual=wait(remote_png_matches);cases.append({'id':'REAL_FEDORA_RDP_LOCAL_TO_REMOTE_PNG','PASS':True,'dimensions':size,'pixel_sha256':local_pixel})
finally:
 try:command('gui','rdp-policy',rdp=0,policy=4)
 except Exception:pass
 try:command('remote','clear');time.sleep(.3);command('remote','quit')
 except Exception:pass
 if peer.poll()is None:peer.terminate();peer.wait(timeout=8)
 receipt={'cases':cases,'real_external_host':'Fedora GNOME authorized account','scope':'Production RDP/GDK/CLIPRDR; synthetic isolated local focus; no physical desktop claim','remote_initial_clipboard_empty':True,'only_generated_payloads':True,'policy':'bidirectional'if a.bidirectional else'directional'};(e/'transfer-receipt.json').write_text(json.dumps(receipt,indent=2)+'\n');print(json.dumps(receipt,indent=2))
