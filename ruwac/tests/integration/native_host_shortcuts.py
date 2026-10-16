#!/usr/bin/env python3
"""Real isolated GTK host menus and GDK chords; no external connection or key grant."""
import argparse,json,os,pwd,subprocess,tempfile,time
from pathlib import Path
from spike import Client,eventually
p=argparse.ArgumentParser();p.add_argument('--binary',required=True);p.add_argument('--gui',required=True);p.add_argument('--evidence',type=Path,required=True);a=p.parse_args()
assert os.environ.get('WAYLAND_DISPLAY','').startswith('ruwac-')
root=Path(tempfile.mkdtemp(prefix='native-host-actions-',dir=a.evidence));root.chmod(0o700)
env={**os.environ,'GTK_A11Y':'none','GSETTINGS_BACKEND':'memory','SHELL':'/bin/bash'}
for key,name in [('HOME','home'),('XDG_CONFIG_HOME','config'),('XDG_DATA_HOME','data'),('XDG_CACHE_HOME','cache')]:env[key]=str(root/name);Path(env[key]).mkdir(mode=0o700)
profile=Path(env['XDG_CONFIG_HOME'])/'RUWAC/machines.json';profile.parent.mkdir(mode=0o700)
def endpoint(i,name,protocol,last_used=0):
 return {'id':'endpoint_demo'+str(i),'machine_id':'machine_demo'+str(i),'name':name,'group':'Generated','protocol':protocol,'server':'127.0.0.1','username':'generated-service'if protocol=='RDP'else'generated-ssh','secret_reference':'generated-reference'+str(i),'favorite':i==0,'last_used':last_used,'tags':[],'preferences':{'codex_command':'codex resume --all','claude_command':'claude --resume','kimi_command':'kimi --session'}}
profile.write_text(json.dumps({'schema':1,'items':[endpoint(0,'Demo','RDP'),endpoint(1,'Recent generated','SSH',20),endpoint(2,'Saved generated','SSH')],'groups':[]}));profile.chmod(0o600)
sock=root/'control.sock';fixture=root/'fixture';fixture.mkdir(mode=0o700);seq=0;children=[];checks=[]
def start(argv,name):
 log=(root/(name+'.log')).open('w');child=subprocess.Popen(argv,env=env,stdout=log,stderr=log);children.append((child,log));return child

def command(action,**kw):
 global seq
 seq+=1;tmp=fixture/'request.tmp';tmp.write_text(json.dumps({'seq':seq,'action':action,**kw}));tmp.replace(fixture/'gui-request.json')
 def read():
  try:q=json.loads((fixture/'gui-response.json').read_text());return q['result']if q['seq']==seq else None
  except(OSError,ValueError):return None
 return eventually(read,12)
try:
 start([a.binary,'--socket',str(sock),'serve'],'daemon');eventually(sock.exists)
 gui=start([a.gui,'--new-instance','--socket',str(sock),'--show-machines','--fixture-dir',str(fixture)],'gui');eventually(lambda:command('inspect').get('target'),20)
 command('chrome',width=1100,height=800,collapse=False);time.sleep(.4)
 widgets=command('host-shortcuts')['widgets'];first={}
 for item in widgets:
  if item['name']in ['machine-connect','machine-terminal','machine-codex','machine-claude','machine-kimi']and item['name']not in first and item['bounds']and item['bounds'][2]>0:first[item['name']]=item
 assert len(first)==5,first
 actions=[v for k,v in first.items() if k!='machine-connect'];centers=[v['bounds'][1]+v['bounds'][3]/2 for v in actions];assert max(centers)-min(centers)<4,first
 assert min(v['bounds'][1] for v in actions)>=first['machine-connect']['bounds'][1]+first['machine-connect']['bounds'][3]-1,first
 checks.append('RDP-bookmark-configured-SSH-Codex-Claude-Kimi-labelled-on-one-line-below-name')
 state=profile.read_bytes();command('host-shortcuts',click='machine-terminal')
 eventually(lambda:any(d['width']>0 for d in command('profile-editor',operation='layout')['dialogs']))
 assert any(x['name']=='ssh-choose-key'for x in command('host-shortcuts')['widgets'])
 command('profile-editor',operation='inspect',ssh_setup=True,response='cancel');assert profile.read_bytes()==state
 checks.append('missing-SSH-opens-explicit-setup-and-cancel-preserves-library')
 command('host-shortcuts',click='machine-terminal');eventually(lambda:any(d['width']>0 for d in command('profile-editor',operation='layout')['dialogs']))
 key=root/'generated-key';key.write_text('Generated inert key selection; never used for login.\n');key.chmod(0o600)
 result=command('profile-editor',operation='inspect',ssh_setup=True,fields={'11':str(key)},response='save');assert not result.get('error'),result
 items=json.loads(profile.read_text())['items'];ssh=next(x for x in items if x['machine_id']=='machine_demo0'and x['protocol']=='SSH');rdp=next(x for x in items if x['id']=='endpoint_demo0')
 assert ssh['username']==pwd.getpwuid(os.getuid()).pw_name and rdp['username']=='generated-service'
 assert ssh['id']!=rdp['id']and ssh['secret_reference']!=rdp['secret_reference'];assert ssh['preferences']['identity_file']==str(key)
 checks.append('linked-SSH-saves-current-user-default-key-path-and-separate-credential-identity')
 time.sleep(.4);header=command('host-shortcuts',operation='header');assert 'Saved agent hosts'in header['sections'];assert 'Running agent sessions'in header['sections'];assert sum(x['name']=='saved-host-codex'for x in header['widgets'])>=3;assert sum(x['name']=='saved-host-claude'for x in header['widgets'])>=3
 assert sum(x['name']=='saved-host-kimi'for x in header['widgets'])>=3
 command('host-shortcuts',close=True)
 menu=command('host-shortcuts',operation='context');assert 'Recent SSH connections'in menu['sections']and'Saved SSH hosts'in menu['sections'];assert menu['sections'].index('Recent SSH connections')<menu['sections'].index('Saved SSH hosts');command('host-shortcuts',close=True)
 checks.append('native-agent-dropdown-and-terminal-context-recent-before-saved-actions')
 command('profile-editor',operation='edit',response='keep');eventually(lambda:any(d['width']>0 for d in command('profile-editor',operation='layout')['dialogs']))
 controls=command('host-shortcuts')['widgets'];assert any(x['name']=='machine-install-ruwac'and x['sensitive']for x in controls);assert any(x['name']=='ssh-endpoint-properties'for x in controls);assert any(x['name']=='ssh-authorize-key'and x['sensitive']for x in controls)
 command('host-shortcuts',click='machine-install-ruwac');command('host-shortcuts',cancel_confirmation=True)
 command('host-shortcuts',click='ssh-authorize-key');command('host-shortcuts',cancel_confirmation=True);command('profile-editor',operation='inspect',response='cancel')
 checks.append('linked-RDP-install-and-public-key-authorization-available-confirmation-cancels-no-execution')
 command('sidebar-shell',shown=False);command('focus-view')
 from gi.repository import Gio,GLib
 bus=Gio.bus_get_sync(Gio.BusType.SESSION,None);dest='org.gnome.Mutter.RemoteDesktop'
 path=bus.call_sync(dest,'/org/gnome/Mutter/RemoteDesktop',dest,'CreateSession',None,None,Gio.DBusCallFlags.NONE,5000,None).unpack()[0]
 def call(method,args=None):return bus.call_sync(dest,path,dest+'.Session',method,args,None,Gio.DBusCallFlags.NONE,5000,None)
 call('Start')
 # Establish the new virtual seat before testing a modifier chord.
 call('NotifyKeyboardKeycode',GLib.Variant('(ub)',(42,True)));call('NotifyKeyboardKeycode',GLib.Variant('(ub)',(42,False)));time.sleep(.15)
 command('focus-view')
 def chord(key):
  for value,state in [(29,True),(42,True),(key,True),(key,False),(42,False),(29,False)]:
   call('NotifyKeyboardKeycode',GLib.Variant('(ub)',(value,state)));time.sleep(.04)
 eventually(lambda:command('inspect')['active']);before=command('inspect')['tabs'];chord(20);eventually(lambda:command('inspect')['tabs']==before+1)
 def keys(*events,gap=.04):
  for value,state in events:call('NotifyKeyboardKeycode',GLib.Variant('(ub)',(value,state)));time.sleep(gap)
 # Remote mode (default): Alt+digit belongs to the application. An AltGr tap enters
 # local mode (Alt+digit selects tabs); the next tap leaves it. AltGr chords never toggle.
 # Linux keycodes: 2 = "1" (tab 0), 3 = "2" (tab 1), 11 = "0" (tab 10), 16 = q, 56 = Alt, 100 = AltGr.
 assert before>=1 and command('inspect')['selected_tab']==before
 keys((56,True),(2,True),(2,False),(56,False));time.sleep(.3);assert command('inspect')['selected_tab']==before
 keys((100,True),(100,False));time.sleep(.2)
 keys((56,True),(2,True),(2,False),(56,False));eventually(lambda:command('inspect')['selected_tab']==0)
 keys((56,True),(11,True),(11,False),(56,False));time.sleep(.3);assert command('inspect')['selected_tab']==0
 keys((100,True),(100,False));time.sleep(.2)
 keys((56,True),(3,True),(3,False),(56,False));time.sleep(.3);assert command('inspect')['selected_tab']==0
 keys((100,True),(16,True),(16,False),(100,False));time.sleep(.2)
 keys((56,True),(3,True),(3,False),(56,False));time.sleep(.3);assert command('inspect')['selected_tab']==0
 checks.append('real-GDK-AltGr-tap-toggles-local-mode-Alt-digit-tabs-chords-do-not-toggle')
 command('focus-view');chord(19);eventually(lambda:command('sidebar-shell')['shown'])
 call('Stop');checks.append('real-GDK-Ctrl-Shift-T-new-terminal-and-Ctrl-Shift-R-machine-chooser')
 command('snapshot')
 command('sidebar-shell',shown=False)
 command('chrome',width=1240,height=880)
 state_path=Path(env['XDG_CONFIG_HOME'])/'RUWAC/window-state.json'
 eventually(lambda:state_path.exists() and json.loads(state_path.read_text())['width']==1240)
 with Client(sock)as c:before_ids={s['id']for s in c.call('session.list',{})}
 saved_tabs=command('inspect')['tabs']
 command('workspace-close');gui.wait(timeout=10)
 saved_state=json.loads(state_path.read_text());assert saved_state['width']==1240 and saved_state['height']==880,saved_state
 assert state_path.stat().st_mode&0o777==0o600
 (fixture/'gui-request.json').unlink(missing_ok=True);(fixture/'gui-response.json').unlink(missing_ok=True)
 gui=start([a.gui,'--new-instance','--socket',str(sock),'--fixture-dir',str(fixture)],'gui-restored')
 eventually(lambda:command('inspect').get('target'),20)
 assert command('inspect')['tabs']==saved_tabs
 with Client(sock)as c:assert {s['id']for s in c.call('session.list',{})}==before_ids
 eventually(lambda:command('chrome')['window_width']==1240 and command('chrome')['window_height']==880)
 command('chrome',maximized=True)
 eventually(lambda:command('chrome')['maximized'])
 command('workspace-close');gui.wait(timeout=10)
 assert json.loads(state_path.read_text())['maximized']
 (fixture/'gui-request.json').unlink(missing_ok=True);(fixture/'gui-response.json').unlink(missing_ok=True)
 gui=start([a.gui,'--new-instance','--socket',str(sock),'--fixture-dir',str(fixture)],'gui-restored-maximized')
 eventually(lambda:command('chrome')['maximized'],20)
 command('chrome',maximized=False)
 eventually(lambda:command('chrome')['window_width']==1240 and command('chrome')['window_height']==880)
 checks.append('native-close-reopen-restores-size-tabs-and-existing-session-identities-without-duplicates')
 command('focus-view',view=0);target=command('inspect')['target']
 with Client(sock)as c:
  candidates=[pane for session in c.call('session.list',{}) for pane in c.call('pane.list',{'session':session['id']})]
  selected=next(pane for pane in candidates if pane['id']==target)
  c.call('pane.create',{'window':selected['window'],'orientation':'vertical','argv':['python3','-c',"import os,time;os.write(1,b'\\x1b[48;2;20;40;60m\\x1b[2J');time.sleep(60)"]})
  c.call('pane.sendText',{'pane':target,'text':"printf '\\033[48;2;20;40;60m\\033[2J'\n"})
 eventually(lambda:command('inspect')['panes'][0]==2)
 time.sleep(.25);info=command('inspect');assert command('snapshot')['saved']
 from PIL import Image
 image=Image.open(fixture/'native-rust.png').convert('RGB')
 x=round(info['area_origin'][0]+(info['grid_size'][0]+1)*info['cell_metrics'][0]);y=round(info['area_origin'][1]+3*info['cell_metrics'][1])
 assert any(all(abs(pixel[i]-[120,171,204][i])<=3 for i in range(3))for pixel in [image.getpixel((x+dx,y))for dx in [-1,0,1]]),'split separator was painted over'
 checks.append('real-GTK-split-separator-survives-both-pane-background-repaints')
 receipt={'status':'PASS','checks':checks,'scope':'Actual GTK/Wayland, synthetic Mutter seat events, generated profiles. No external login, installer or key authorization executed.'}
 (root/'acceptance.json').write_text(json.dumps(receipt,indent=2)+'\n');print('Native host shortcuts PASS',len(checks),'groups; evidence',root)
finally:
 for child,log in reversed(children):
  if child.poll()is None:child.terminate()
  try:child.wait(timeout=8)
  except subprocess.TimeoutExpired:child.kill();child.wait(timeout=5)
  log.close()
