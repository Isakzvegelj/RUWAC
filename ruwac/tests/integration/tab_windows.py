#!/usr/bin/env python3
"""Native tab transfer keeps live PTYs and an authenticated loopback RDP transport."""
import argparse,json,os,shutil,subprocess,tempfile,time
from pathlib import Path
from spike import Client,eventually
p=argparse.ArgumentParser()
for name in ['binary','gui','server','evidence']:p.add_argument('--'+name,required=True)
a=p.parse_args();assert os.environ.get('WAYLAND_DISPLAY')=='ruwac-test'
root=Path(tempfile.mkdtemp(prefix='windows-',dir=a.evidence));root.chmod(0o700);env={**os.environ,'GTK_A11Y':'none','GSETTINGS_BACKEND':'memory','LP_NUM_THREADS':'2','RAYON_NUM_THREADS':'2'}
for key,leaf in [('HOME','home'),('XDG_CONFIG_HOME','config'),('XDG_DATA_HOME','data'),('XDG_CACHE_HOME','cache')]:
 (root/leaf).mkdir(mode=0o700);env[key]=str(root/leaf)
(root/'config/RUWAC').mkdir(mode=0o700);sock=root/'control.sock';gui_sock=sock.with_suffix('.gui.sock');seq=0;children=[];checks=[]
def load(path):
 try:return json.loads(path.read_text())
 except(OSError,ValueError):return {}
def start(args,name):
 child=subprocess.Popen(list(map(str,args)),env=env,stdout=(root/(name+'.log')).open('wb'),stderr=subprocess.STDOUT);children.append(child);return child
def rpc(path,method,params=None):
 with Client(path)as c:return c.call(method,params)
def command(action,**kw):
 global seq
 seq+=1;temporary=root/'request.tmp';temporary.write_text(json.dumps(dict(seq=seq,action=action,**kw)));temporary.replace(root/'gui-request.json')
 return eventually(lambda:v['result']if(v:=load(root/'gui-response.json')).get('seq')==seq else None,12)
def listing():return rpc(gui_sock,'workspace.list')
def panes():return {p['id']:p['pid']for s in rpc(sock,'session.list')for p in rpc(sock,'pane.list',{'session':s['id']})}
def mark(name):checks.append(name);print(name,'PASS',flush=True)
try:
 start([a.binary,'--socket',sock,'serve'],'daemon');eventually(sock.exists)
 sessions=[rpc(sock,'session.create',{'name':'Generated '+str(i),'argv':['/bin/bash','--noprofile','--norc','-c','printf "GENERATED_READY\\n"; exec sleep 600']})['session']for i in range(2)]
 baseline=panes()
 gui=start([a.gui,'--new-instance','--socket',sock,'--session',sessions[0],'--session',sessions[1],'--allow-workspace-control','--fixture-dir',root],'GUI');eventually(lambda:command('inspect').get('tabs')==2,15)
 # Exercise production Store publication before opening a connection. Existing
 # secret references, names and unrelated preferences must survive the update.
 old={'schema':1,'items':[{'id':'endpoint_generated_bulk','machine_id':'machine_generated_bulk','name':'Generated','group':'','protocol':'RDP','server':'127.0.0.1','username':'fixture','secret_reference':'GENERATED_KEEP','favorite':True,'last_used':12,'tags':['retained'],'preferences':{'display':'fit','zoom':1.25,'clipboard':4}}],'groups':[{'id':'group_generated','path':'Generated group','defaults':{'display':'native','zoom':1.5,'clipboard':2}}]}
 # Use a separate private config so the running GUI's canonical store is untouched.
 bulk=root/'bulk';(bulk/'RUWAC').mkdir(parents=True,mode=0o700);path=bulk/'RUWAC/machines.json';path.write_text(json.dumps(old));path.chmod(0o600)
 for _ in range(2):subprocess.run([a.gui,'--dynamic-rdp'],env=dict(env,XDG_CONFIG_HOME=str(bulk)),check=True,stdout=subprocess.DEVNULL)
 assert load(path)['groups'][0]['defaults']['display']=='dynamic' and load(path)['groups'][0]['defaults']['zoom']==1.5 and load(path)['groups'][0]['defaults']['clipboard']==2
 updated=load(path)['items'][0];assert updated['preferences']['display']=='dynamic'
 for key,value in old['items'][0].items():
  if key!='preferences':assert updated[key]==value
 for key,value in old['items'][0]['preferences'].items():
  if key!='display':assert updated['preferences'][key]==value
 mark('bulk-dynamic-update-preserves-credentials-and-unrelated-settings-and-is-idempotent')
 server_dir=root/'server';server_dir.mkdir(mode=0o700)
 subprocess.run(['openssl','req','-x509','-newkey','rsa:2048','-nodes','-days','1','-subj','/CN=generated-loopback','-addext','subjectAltName=IP:127.0.0.1','-keyout',str(server_dir/'key.pem'),'-out',str(server_dir/'cert.pem')],check=True,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL);(server_dir/'key.pem').chmod(0o600)
 shutil.copy2(server_dir/'cert.pem',root/'cert.pem')
 start([a.server,server_dir],'RDP-server');port=eventually(lambda:load(server_dir/'server-ready.json').get('ports'),12)[0]
 command('rdp-open',port=port);eventually(lambda:command('rdp-inspect').get('metrics',{}).get('decoded_updates',0)>0,15)
 state=listing();ids={v['id']for v in state['views']};rdp=next(v['id']for v in state['views']if v['kind']=='rdp');connection=rpc(gui_sock,'rdp.query',{'id':rdp});assert connection['transport_id'];transport=connection['transport_id']
 # Invoke the real tab context-menu action, which uses create-window, then the
 # same native transfer_page operation used by an external tab drag.
 command('rdp-tab-menu',tab=2,control='detach-tab');eventually(lambda:len(listing()['windows'])==2)
 now=rpc(gui_sock,'rdp.query',{'id':rdp});assert now['transport_id']==transport and now['phase']=='connected';assert {v['id']for v in listing()['views']}==ids and panes()==baseline
 mark('native-create-window-transfer-preserves-RDP-transport-PTY-PIDs-and-all-view-IDs')
 snapshot=rpc(gui_sock,'workspace.save',{'name':'Generated detached workspace'})['id']
 with Client(gui_sock)as c:c.call('workspace.restore',{'id':snapshot},error='WORKSPACE_CONFLICT')
 assert rpc(gui_sock,'rdp.query',{'id':rdp})['transport_id']==transport and panes()==baseline
 mark('saved-layout-refuses-to-displace-live-detached-views')
 focused=rpc(gui_sock,'workspace.focus',{'id':rdp});assert focused['focused_view']==rdp
 # A synthetic API request has no Wayland user activation serial. Selection
 # must address the transferred view; actual seat activation is verified below
 # with a real compositor pointer click, rather than faking window_active.
 mark('transferred-RDP-remains-addressable-through-authenticated-GUI-API')
 # Real compositor pointer events must reach the reparented DrawingArea,
 # including after a negotiated resize. No fixture pointer method is used.
 from gi.repository import Gio,GLib
 bus=Gio.bus_get_sync(Gio.BusType.SESSION,None);service='org.gnome.Mutter.RemoteDesktop'
 input_path=bus.call_sync(service,'/org/gnome/Mutter/RemoteDesktop',service,'CreateSession',None,None,Gio.DBusCallFlags.NONE,5000,None).unpack()[0]
 def native(method,args=None):return bus.call_sync(service,input_path,service+'.Session',method,args,None,Gio.DBusCallFlags.NONE,5000,None)
 native('Start')
 # Bind the virtual pointer before the keyboard activation and first click.
 native('NotifyPointerMotionRelative',GLib.Variant('(dd)',(1.,1.)))
 try:
  # The headless compositor starts without an active keyboard window. Use its
  # actual local window-switch shortcut, as in the two-desktop clipboard test.
  for _ in range(6):
   rpc(gui_sock,'workspace.focus',{'id':rdp})
   if command('rdp-inspect')['gnome_auto']['guards']['window_active']:break
   for code,pressed in [(56,True),(15,True),(15,False),(56,False)]:
    native('NotifyKeyboardKeycode',GLib.Variant('(ub)',(code,pressed)));time.sleep(.04)
   time.sleep(.15)
  eventually(lambda:command('rdp-inspect')['gnome_auto']['guards']['window_active'])
  command('rdp-toolbar',control='fullscreen');eventually(lambda:command('rdp-inspect')['window_width']==1200)
  def click():
   before=load(server_dir/('input-'+str(port)+'.mouse.json')).get('events',[]).count('LeftReleased')
   native('NotifyPointerMotionRelative',GLib.Variant('(dd)',(-10000.,-10000.)));time.sleep(.04)
   native('NotifyPointerMotionRelative',GLib.Variant('(dd)',(600.,400.)));time.sleep(.04)
   native('NotifyPointerButton',GLib.Variant('(ib)',(272,True)));time.sleep(.06)
   native('NotifyPointerButton',GLib.Variant('(ib)',(272,False)))
   eventually(lambda:load(server_dir/('input-'+str(port)+'.mouse.json')).get('events',[]).count('LeftReleased')==before+1)
  click()
  try:eventually(lambda:listing()['input_focus_view']==rdp)
  except AssertionError:
   (root/'focus-failure.json').write_text(json.dumps({'workspace':listing(),'native':command('rdp-inspect')},indent=2))
   raise
  command('rdp-display',mode='dynamic',zoom=1.)
  eventually(lambda:(v:=rpc(gui_sock,'rdp.query',{'id':rdp}))['size']==v['viewport']and v['size'][0]>=1100,15)
  click();assert rpc(gui_sock,'rdp.query',{'id':rdp})['transport_id']==transport
  mark('native-GDK-pointer-works-before-and-after-negotiated-dynamic-resize-in-new-window')
  command('rdp-toolbar',control='fullscreen')
 finally:native('Stop')
 command('detach-tab',operation='return',window=1);eventually(lambda:command('inspect')['tabs']==3);assert rpc(gui_sock,'rdp.query',{'id':rdp})['transport_id']==transport
 mark('returning-a-live-tab-preserves-its-original-connection')
 rpc(gui_sock,'workspace.restore',{'id':snapshot});assert rpc(gui_sock,'rdp.query',{'id':rdp})['transport_id']==transport
 # Transfer a mixed split tree; each leaf remains exactly the same live view.
 command('tab-select',tab=0);command('mixed-join',tab=2,vertical=True);command('rdp-tab-menu',tab=0,control='detach-tab');assert panes()==baseline and {v['id']for v in listing()['views']}==ids
 mark('mixed-terminal-RDP-split-transfers-as-one-tab-without-reconnecting')
 # Close only the original application window. The transferred tree and API
 # must continue serving while the other persistent PTY also remains alive.
 command('workspace-close');time.sleep(.3);assert gui.poll()is None and panes()==baseline
 assert rpc(gui_sock,'rdp.query',{'id':rdp})['transport_id']==transport
 mark('closing-original-window-keeps-detached-window-RDP-and-daemon-PTYs-alive')
 eventually(lambda:len(listing()['views'])==2)
 def saved_survivors():
  for path in(root/'config/RUWAC').rglob('workspace*.json'):
   snapshot=load(path)
   if snapshot.get('socket')==str(sock)and len(snapshot.get('sessions',[]))==1 and snapshot.get('tabs'):
    return snapshot
  return None
 saved=eventually(saved_survivors,5);assert sessions[0]in saved['sessions'] and sessions[1]not in saved['sessions']
 mark('autosave-keeps-detached-tabs-and-excludes-closed-original-frontends')
 (root/'acceptance.json').write_text(json.dumps({'PASS':True,'checks':checks,'RDP':'authenticated generated loopback','native_transfer':True,'physical_drag':False,'owner_desktop_used':False},indent=2)+'\n')
finally:
 for child in reversed(children):
  if child.poll()is None:child.terminate()
 for child in reversed(children):
  try:child.wait(timeout=5)
  except subprocess.TimeoutExpired:child.kill();child.wait()
 print(root,flush=True)
