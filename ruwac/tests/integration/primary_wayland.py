#!/usr/bin/env python3
"""Real PRIMARY transport with synthetic GTK dispatch on an isolated Mutter display."""
from pathlib import Path
import argparse,json,os,subprocess,sys,time,hashlib,tempfile,re,signal
from gi.repository import Gio,GLib
parser=argparse.ArgumentParser(description='Generated PRIMARY and GTK click-dispatch acceptance on an owned Mutter display');parser.add_argument('--binary',required=True);parser.add_argument('--gui',required=True);parser.add_argument('--evidence',type=Path,required=True);mode=parser.add_mutually_exclusive_group(required=True);mode.add_argument('--owned-display',action='store_true');mode.add_argument('--spawn-owned-display',action='store_true');args=parser.parse_args();r=args.evidence;r.mkdir(mode=0o700,parents=True,exist_ok=True);env={**os.environ,'GDK_BACKEND':'wayland','GSETTINGS_BACKEND':'memory','GTK_A11Y':'none','WAYLAND_DEBUG':'1'}
if args.spawn_owned_display:
 display_root=Path(tempfile.mkdtemp(prefix='mutter-display-',dir=r));display_root.chmod(0o700)
 for key,name in [('HOME','home'),('XDG_CONFIG_HOME','config'),('XDG_DATA_HOME','data'),('XDG_CACHE_HOME','cache'),('XDG_RUNTIME_DIR','runtime')]:
  env[key]=str(display_root/name);Path(env[key]).mkdir(mode=0o700)
 for key in ['DISPLAY','WAYLAND_DISPLAY','DBUS_SESSION_BUS_ADDRESS','RUWAC_XTEST_DISPLAY']:
  env.pop(key,None)
 display_name='ruwac-test-primary';bus_file=display_root/'bus';log=(display_root/'compositor.log').open('w')
 compositor=subprocess.Popen(['dbus-run-session','--','sh','-c','printf "%s" "$DBUS_SESSION_BUS_ADDRESS" > "$1"; exec mutter --headless --wayland --no-x11 --virtual-monitor 1280x800 --wayland-display ruwac-test-primary','owned-compositor',str(bus_file)],env=env,stdout=log,stderr=log,start_new_session=True)
 try:
  for _ in range(100):
   if compositor.poll()is not None:raise RuntimeError('Owned compositor failed; inspect its private log')
   if (Path(env['XDG_RUNTIME_DIR'])/display_name).exists()and bus_file.exists():break
   time.sleep(.1)
  else:raise TimeoutError('Owned compositor socket did not appear')
  env.update({'WAYLAND_DISPLAY':display_name,'DBUS_SESSION_BUS_ADDRESS':bus_file.read_text()})
  result=subprocess.run([sys.executable,str(Path(__file__).resolve()),'--binary',args.binary,'--gui',args.gui,'--evidence',str(r),'--owned-display'],env=env)
 finally:
  if compositor.poll()is None:os.killpg(compositor.pid,signal.SIGTERM)
  try:compositor.wait(timeout=10)
  except subprocess.TimeoutExpired:os.killpg(compositor.pid,signal.SIGKILL);compositor.wait()
  log.close()
 raise SystemExit(result.returncode)
os.environ['DBUS_SESSION_BUS_ADDRESS']=env['DBUS_SESSION_BUS_ADDRESS']
root=Path(tempfile.mkdtemp(prefix='acceptance-',dir=r));root.chmod(0o700);sock=root/'control.sock';children=[];seq={}
assert env.get('WAYLAND_DISPLAY','').startswith('ruwac-'), 'An explicitly owned test Wayland display is required'
assert Path(env['XDG_RUNTIME_DIR']).resolve()!=Path('/run/user')/str(os.getuid()), 'Never run this fixture on the normal desktop runtime'
for key,name in [('HOME','home'),('XDG_CONFIG_HOME','config'),('XDG_DATA_HOME','data'),('XDG_CACHE_HOME','cache')]:
 env[key]=str(root/name);Path(env[key]).mkdir(mode=0o700)
sys.path.insert(0,str(Path(__file__).resolve().parent));from spike import Client,eventually
bus=Gio.bus_get_sync(Gio.BusType.SESSION,None);dest='org.gnome.Mutter.RemoteDesktop';interface=dest+'.Session'
path=bus.call_sync(dest,'/org/gnome/Mutter/RemoteDesktop',dest,'CreateSession',None,None,Gio.DBusCallFlags.NONE,5000,None).unpack()[0]
def call(method,args=None):return bus.call_sync(dest,path,interface,method,args,None,Gio.DBusCallFlags.NONE,5000,None)
call('Start')
def key(n,state):call('NotifyKeyboardKeysym',GLib.Variant('(ub)',(n,state)))
def toggle():
 key(65513,True);key(65289,True);key(65289,False);key(65513,False);time.sleep(.3)
def start(argv,name):
 f=(root/(name+'.log')).open('w');p=subprocess.Popen(argv,env=env,stdout=f,stderr=f);children.append((p,f));return p

def cmd(who,action,**kw):
 seq[who]=seq.get(who,0)+1;n=seq[who];tmp=root/(who+'-request.tmp');tmp.write_text(json.dumps({'seq':n,'action':action,**kw}));tmp.replace(root/(who+'-request.json'))
 def read():
  try:v=json.loads((root/(who+'-response.json')).read_text());return v['result']if v['seq']==n else None
  except (ValueError,OSError):return None
 result=eventually(read,7)
 with (root/'commands.jsonl').open('a')as out:out.write(json.dumps({'who':who,'action':action,'args':kw,'result':result})+'\n')
 return result
def focus(who):
 for _ in range(20):
  focused=cmd('gui','inspect')['active']if who=='gui'else cmd('peer','inspect')['window_active']
  if focused:
   key(65505,True);key(65505,False);time.sleep(.1)
   if who=='gui':cmd('gui','focus-view',view=0)
   return
  toggle()
 raise AssertionError('Real Mutter focus switching failed')
def rpc(m,p=None):
 with Client(sock)as c:return c.call(m,p)
try:
 start([args.binary,'--socket',str(sock),'serve'],'daemon');eventually(lambda:sock.exists())
 pane=rpc('session.create',{'name':'Generated PRIMARY','argv':[sys.executable,str(Path(__file__).resolve().parent.parent/'fixtures/receiver.py'),str(root/'received')]})
 rpc('pane.wait',{'pane':pane['pane'],'contains':'READY','timeout_ms':5000})
 start([sys.executable,str(Path(__file__).resolve().parent.parent/'fixtures/desktop_clipboard.py'),str(root)],'peer');eventually(lambda:(root/'peer-ready.json').exists())
 start([args.gui,'--socket',str(sock),'--session',pane['session'],'--fixture-dir',str(root)],'gui');eventually(lambda:cmd('gui','inspect'),15)
 normal=b'GENERATED_CLIPBOARD_B';(root/'clipboard.txt').write_bytes(normal);focus('peer');cmd('peer','set-text',file='clipboard.txt');time.sleep(.2)
 focus('gui');before=cmd('gui','read-text');assert before.get('sha256')==hashlib.sha256(normal).hexdigest(),{'before_PRIMARY':before}
 expected="Native selection žółć 🧪\nline\t'quotes'".encode();focus('gui');cmd('gui','copy',primary=True);time.sleep(.2);focus('peer');v=cmd('peer','read-text',primary=True);assert v['sha256']==hashlib.sha256(expected).hexdigest(),v
 b=cmd('peer','read-text');assert b.get('sha256')==hashlib.sha256(normal).hexdigest(),b
 rpc('buffer.set',{'session':pane['session'],'text':'GENERATED_INTERNAL_A'})
 primary='GENERATED_PRIMARY_P žółć\nsecond'.encode();(root/'primary.txt').write_bytes(primary);cmd('peer','set-text',primary=True,file='primary.txt');time.sleep(.2)
 focus('gui');cmd('gui','key',key='Escape');cmd('gui','click',button=2,x=20.0,y=20.0)
 eventually(lambda:primary in (root/'received').read_bytes(),7)
 rpc('pane.sendText',{'pane':pane['pane'],'text':'/mouse-on/'})
 time.sleep(.3);before=(root/'received').read_bytes();cmd('gui','click',button=2,x=20.0,y=20.0);cmd('gui','click',button=2,x=20.0,y=20.0,release=True)
 def routed():
  delta=(root/'received').read_bytes()[len(before):];return delta if re.fullmatch(rb'\x1b\[<1;[0-9]+;[0-9]+M\x1b\[<1;[0-9]+;[0-9]+m',delta) else None
 routed_bytes=eventually(routed,7)
 before=(root/'received').read_bytes();cmd('gui','click',button=2,x=20.0,y=20.0,mods=1);cmd('gui','click',button=2,x=20.0,y=20.0,mods=1,release=True)
 eventually(lambda:(root/'received').read_bytes()==before+primary,7)
 rpc('pane.sendText',{'pane':pane['pane'],'text':'/mouse-off/'})
 time.sleep(.3);before=(root/'received').read_bytes();cmd('gui','click',button=1,x=20.0,y=20.0);cmd('gui','click',button=1,x=20.0,y=20.0,release=True);time.sleep(.2);assert (root/'received').read_bytes()==before
 assert rpc('buffer.get',{'session':pane['session']})['text']=='GENERATED_INTERNAL_A'
 focus('peer');b=cmd('peer','read-text');assert b.get('sha256')==hashlib.sha256(normal).hexdigest(),b
 assert 'zwp_primary_selection_device_manager_v1' in (root/'peer.log').read_text()
 print('Real Mutter PRIMARY copy, middle-click handler paste and SGR application routing with Shift bypass and independent clipboard/Screen buffers PASS',flush=True)
 (r/'primary-copy.json').write_text(json.dumps({'status':'PASS','compiled_GTK':True,'real_primary_protocol':True,'real_compositor_focus_keyboard':True,'physical_input':False,'sha256':v['sha256'],'version':subprocess.check_output([args.gui,'--version'],text=True).strip(),'middle_click_handler_paste':True,'application_middle_press_release':True,'shift_middle_PRIMARY_bypass':True,'disabled_mouse_no_leak':True,'compositor':subprocess.check_output(['mutter','--version'],text=True).strip(),'protocol':'zwp_primary_selection_device_manager_v1','source_commit':os.environ.get('RUWAC_ACCEPTANCE_COMMIT','uncommitted candidate'),'CLIPBOARD_B_preserved':True,'Screen_A_preserved':True},indent=2))
finally:
 try:call('Stop')
 except Exception:pass
 for p,f in reversed(children):
  if p.poll()is None:p.terminate()
  try:p.wait(timeout=10)
  except subprocess.TimeoutExpired:p.kill();p.wait()
  f.close()
