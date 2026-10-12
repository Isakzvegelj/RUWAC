#!/usr/bin/env python3
"""Real authorized SSH plus owned Unix forwarding to a private native GTK fixture.
The fixture renders on the local owned compositor, not the remote physical seat.
"""
import argparse,hashlib,json,os,shlex,subprocess,tempfile,time,uuid
from pathlib import Path
from spike import Client,eventually
p=argparse.ArgumentParser();p.add_argument('--binary',required=True);p.add_argument('--gui',required=True);p.add_argument('--host',required=True);p.add_argument('--remote-root',required=True);p.add_argument('--evidence',type=Path,required=True);a=p.parse_args()
root=Path(tempfile.mkdtemp(prefix='remote-gui-',dir=a.evidence));root.chmod(0o700);remote_root=a.remote_root.rstrip('/')+'/fixture-'+uuid.uuid4().hex;remote_bin=a.remote_root.rstrip('/')+'/ruwac-candidate';remote_sock=remote_root+'/control.sock';remote_gui=remote_root+'/control.gui.sock';sock=root/'control.sock';gui_sock=sock.with_suffix('.gui.sock');seq=0;gui=None;daemon=None;tunnel=None;results=[]
ssh=['ssh','-o','BatchMode=yes','-o','StrictHostKeyChecking=yes','-o','ConnectTimeout=10','-o','ForwardAgent=no','--',a.host]
def remote(args):return subprocess.check_output(ssh+[shlex.join(args)],text=True,timeout=25)
def api(namespace,*args,ok=True):
 cmd=[a.binary,'--host',a.host,'--remote-binary',remote_bin,'--socket',remote_sock,namespace,*args,'--json'];r=subprocess.run(cmd,capture_output=True,text=True,timeout=30)
 assert(r.returncode==0)==ok,(cmd,r.stdout,r.stderr);v=json.loads(r.stdout);assert v['version']==0 and v['id'];return v.get('result',v.get('error'))
def command(action,**kw):
 global seq
 seq+=1;t=root/'request.tmp';t.write_text(json.dumps({'seq':seq,'action':action,**kw}));t.replace(root/'gui-request.json')
 def read():
  try:v=json.loads((root/'gui-response.json').read_text());return v['result']if v['seq']==seq else None
  except(OSError,ValueError):return None
 return eventually(read,10)
env={**os.environ,'GTK_A11Y':'none','GSETTINGS_BACKEND':'memory'}
for k,name in [('HOME','home'),('XDG_CONFIG_HOME','config'),('XDG_CACHE_HOME','cache'),('XDG_DATA_HOME','data')]:env[k]=str(root/name);Path(env[k]).mkdir(mode=0o700)
try:
 remote(['mkdir','-p','-m','700',a.remote_root,remote_root]);subprocess.run(['scp','-q',a.binary,a.host+':'+remote_bin],check=True,timeout=40);remote(['chmod','700',remote_bin])
 expected=hashlib.sha256(Path(a.binary).read_bytes()).hexdigest();assert remote(['sha256sum',remote_bin]).split()[0]==expected
 assert api('workspace','list',ok=False)['code']=='GUI_UNAVAILABLE'
 remote(['test','!','-S',remote_sock]);results.append({'id':'REMOTE-GUI-NO-AUTOSTART','status':'PASS','actual_authorized_SSH':True,'missing_GUI_typed_error':True,'no_remote_daemon_started':True})
 daemon=subprocess.Popen([a.binary,'--socket',str(sock),'serve'],env=env,stdout=(root/'daemon.log').open('wb'),stderr=subprocess.STDOUT);eventually(lambda:sock.exists())
 with Client(sock)as c:session=c.call('session.create',{'name':'Remote GUI fixture','argv':['/bin/sh','-c','sleep 300']})
 gui=subprocess.Popen([a.gui,'--socket',str(sock),'--session',session['session'],'--fixture-dir',str(root)],env=env,stdout=(root/'gui.log').open('wb'),stderr=subprocess.STDOUT);eventually(lambda:gui_sock.exists()and command('inspect'),15)
 tunnel=subprocess.Popen(['ssh','-NT','-o','BatchMode=yes','-o','StrictHostKeyChecking=yes','-o','ConnectTimeout=10','-o','ForwardAgent=no','-o','ExitOnForwardFailure=yes','-o','StreamLocalBindMask=0177','-R',remote_gui+':'+str(gui_sock),'--',a.host],stdout=(root/'tunnel.log').open('wb'),stderr=subprocess.STDOUT)
 def tunnel_ready():
  assert tunnel.poll()is None,'Owned SSH forward terminated before becoming ready'
  try:return remote(['test','-S',remote_gui])==''
  except subprocess.CalledProcessError as e:
   if e.returncode!=1:raise
   return False
 eventually(tunnel_ready,15)
 state=api('workspace','list');view=state['views'][0]['id'];assert state['views'][0]['session']==session['session'];description=api('workspace','describe');assert description['transport']['same_uid']and not description['permissions']['rdp_lifecycle_control']
 assert api('rdp','list')['views']==[]
 command('profile-editor',fields={'0':'Remote metadata','3':'metadata.invalid','4':'fixture','9':'PRIVATE_REF_SENTINEL'},response='save')
 hosts=api('host','list');assert hosts['total']==1 and 'PRIVATE_REF_SENTINEL'not in json.dumps(hosts)
 endpoint=hosts['endpoints'][0];assert api('host','query','--id',endpoint['id'])['endpoint']==endpoint
 assert api('workspace','focus','--id',view,ok=False)['code']=='PERMISSION_DENIED';command('workspace-permission',enabled=True);api('workspace','focus','--id',view);assert api('workspace','list')['focused_view']==view
 assert api('rdp','disconnect','--id',view,ok=False)['code']=='PERMISSION_DENIED';command('rdp-permission',enabled=True);assert api('rdp','disconnect','--id',view,ok=False)['code']=='STALE_TARGET'
 command('workspace-permission',enabled=False);assert api('workspace','focus','--id',view,ok=False)['code']=='PERMISSION_DENIED'
 results.append({'id':'REMOTE-GUI-AUTHENTICATED-FORWARD','status':'PASS','actual_SSH_and_Unix_socket_forward':True,'same_UID_checks_retained':True,'metadata_and_exact_view_focus':True,'read_only_host_inventory_and_exact_query':True,'runtime_permission_revocation':True,'RDP_control_separate_permission':True,'local_owned_GTK_rendering':True,'remote_desktop_rendering_claim':False})
 tunnel.terminate();tunnel.wait(timeout=10);tunnel=None
 assert api('workspace','list',ok=False)['code']=='GUI_UNAVAILABLE';remote(['test','!','-S',remote_sock]);results.append({'id':'REMOTE-GUI-DISCONNECT-NO-FALLBACK','status':'PASS','closed_forward_typed_error':True,'no_implicit_launch_or_target_change':True})
finally:
 if tunnel is not None and tunnel.poll()is None:tunnel.terminate();tunnel.wait(timeout=10)
 if gui is not None and gui.poll()is None:gui.terminate();gui.wait(timeout=10)
 if daemon is not None and daemon.poll()is None:
  try:
   with Client(sock)as c:c.call('daemon.stop')
   daemon.wait(timeout=5)
  except Exception:daemon.kill();daemon.wait()
 # The UUID directory was created only by this fixture; preserve the candidate binary.
 try:remote(['python3','-c',"import pathlib,stat,os; p=pathlib.Path(__import__('sys').argv[1]); s=p/'control.gui.sock'; m=s.lstat() if s.exists() else None; assert m is None or (stat.S_ISSOCK(m.st_mode) and m.st_uid==os.getuid()); s.unlink(missing_ok=True); p.rmdir()",remote_root])
 except Exception as e:print('Owned remote socket cleanup failed:',type(e).__name__)
 receipt={'results':results,'hashes':{n:hashlib.sha256(Path(v).read_bytes()).hexdigest()for n,v in [('ruwac',a.binary),('ruwac-desktop',a.gui)]},'evidence':str(root),'scope':'Real authorized SSH via owned Unix socket forwarding to the local isolated GTK compositor; no physical remote desktop or external RDP login claim'}
 (root/'receipt.json').write_text(json.dumps(receipt,indent=2)+'\n');print(json.dumps(receipt,indent=2))
assert len(results)==3
