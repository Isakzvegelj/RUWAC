#!/usr/bin/env python3
"""Real isolated GTK host menus and GDK chords; no external connection or key grant."""
import argparse,json,os,pwd,subprocess,tempfile,time,shutil
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
items=[endpoint(0,'Demo','RDP'),endpoint(1,'Recent generated','SSH',20),endpoint(2,'Saved generated','SSH')]
for index,name,host in [(3,'ZZ verified','verified.generated.test'),(4,'ZZ unknown trust','unknown.generated.test'),(5,'ZZ rejected key','rejected.generated.test')]:
 item=endpoint(index,name,'RDP');item['server']=host;items.append(item)
profile.write_text(json.dumps({'schema':1,'items':items,'groups':[]}));profile.chmod(0o600)
# A generated SSH peer models only the auto-probe boundary. Other SSH calls
# remain real; actual key/trust interoperability is covered by ssh_access.py.
wrappers=root/'bin';wrappers.mkdir();trace=root/'auto-ssh.jsonl'
wrapper=wrappers/'ssh';real=shutil.which('ssh')
wrapper.write_text('#!/usr/bin/python3\nimport os,sys,json\na=sys.argv[1:]\nh=a[a.index("--")+1]if"--"in a else""\nif "BatchMode=yes"in a and h.endswith(".generated.test"):\n with open('+repr(str(trace))+',"a")as f:f.write(json.dumps(a)+"\\n")\n if h=="verified.generated.test":sys.exit(0)\n print("Host key verification failed"if h.startswith("unknown")else"Permission denied (publickey)",file=sys.stderr);sys.exit(255)\nos.execv('+repr(real)+',["ssh"]+a)\n')
wrapper.chmod(0o755);env['PATH']=str(wrappers)+os.pathsep+env['PATH']
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
 eventually(lambda:any(x['machine_id']=='machine_demo3'and x['protocol']=='SSH'for x in json.loads(profile.read_text())['items']),15)
 auto_items=json.loads(profile.read_text())['items']
 assert not any(x['protocol']=='SSH'and x['machine_id']in ['machine_demo4','machine_demo5']for x in auto_items)
 auto=next(x for x in auto_items if x['machine_id']=='machine_demo3'and x['protocol']=='SSH')
 assert auto['username']=='' and auto['secret_reference']!='generated-reference3'
 calls=[json.loads(line)for line in trace.read_text().splitlines()]
 assert all('StrictHostKeyChecking=yes'in c and 'PasswordAuthentication=no'in c and c[-1]=='true'for c in calls)
 checks.append('generated-SSH-probe-auto-adds-only-verified-bookmark-no-unknown-trust-password-or-key-install')

 # Key verification can rebuild rows asynchronously; wait for allocated widgets.
 def allocated_row():
  first={}
  for item in command('host-shortcuts')['widgets']:
   if item['name']in ['machine-connect','machine-terminal','machine-codex','machine-claude','machine-kimi']and item['name']not in first and item['bounds']and min(item['bounds'][2:])>=16:first[item['name']]=item
  return first if len(first)==5 else None
 first=eventually(allocated_row,10)
 actions=[v for k,v in first.items() if k!='machine-connect'];centers=[v['bounds'][1]+v['bounds'][3]/2 for v in actions];assert max(centers)-min(centers)<4,first
 assert min(v['bounds'][1] for v in actions)>=first['machine-connect']['bounds'][1]+first['machine-connect']['bounds'][3]-1,first
 checks.append('RDP-bookmark-configured-SSH-Codex-Claude-Kimi-labelled-on-one-line-below-name')
 # A connect icon connects: no setup form. The machine gets an SSH endpoint from its
 # address (account, port and route left to ~/.ssh/config) and a terminal opens.
 tabs_before=command('inspect')['tabs'];command('host-shortcuts',click='machine-terminal')
 eventually(lambda:any(x['machine_id']=='machine_demo0'and x['protocol']=='SSH'for x in json.loads(profile.read_text())['items']))
 eventually(lambda:command('inspect')['tabs']==tabs_before+1)
 items=json.loads(profile.read_text())['items'];ssh=next(x for x in items if x['machine_id']=='machine_demo0'and x['protocol']=='SSH');rdp=next(x for x in items if x['id']=='endpoint_demo0')
 assert ssh['username']=='' and rdp['username']=='generated-service'
 assert ssh['id']!=rdp['id']and ssh['secret_reference']!=rdp['secret_reference'];assert not ssh.get('preferences',{}).get('identity_file')
 checks.append('connect-icon-adds-ssh-endpoint-from-config-and-opens-terminal-without-setup-form')
 time.sleep(.4);header=command('host-shortcuts',operation='header');assert 'Saved agent hosts'in header['sections'];assert 'Agent sessions'in header['sections'];assert sum(x['name']=='saved-host-codex'for x in header['widgets'])>=3;assert sum(x['name']=='saved-host-claude'for x in header['widgets'])>=3
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
 # Default: Alt+1…9 switch RUWAC tabs. An AltGr tap turns Map keys on (Alt+digit goes to the
 # program); the next tap turns it off. AltGr chords never toggle.
 # Linux keycodes: 2 = "1" (tab 0), 3 = "2" (tab 1), 16 = q, 56 = Alt, 100 = AltGr.
 assert before>=1
 keys((56,True),(2,True),(2,False),(56,False));eventually(lambda:command('inspect')['selected_tab']==0)
 keys((100,True),(100,False));time.sleep(.2)
 keys((56,True),(3,True),(3,False),(56,False));time.sleep(.3);assert command('inspect')['selected_tab']==0
 keys((100,True),(16,True),(16,False),(100,False));time.sleep(.2)
 keys((56,True),(3,True),(3,False),(56,False));time.sleep(.3);assert command('inspect')['selected_tab']==0
 keys((100,True),(100,False));time.sleep(.2)
 keys((56,True),(3,True),(3,False),(56,False));eventually(lambda:command('inspect')['selected_tab']==1)
 checks.append('real-GDK-Alt-digit-switches-tabs-by-default-Map-keys-sends-to-program-chords-do-not-toggle')
 # New host shortcuts use real GDK input, even with collapsed header actions.
 command('chrome',collapse=True);command('sidebar-shell',shown=False);command('focus-view')
 keys((56,True),(50,True),(50,False),(56,False)) # Alt+M
 eventually(lambda:command('sidebar-shell')['shown'])
 command('sidebar-shell',shown=False);command('focus-view')
 keys((56,True),(30,True),(30,False),(56,False)) # Alt+A
 eventually(lambda:'Saved agent hosts' in command('host-shortcuts')['sections'])
 keys((1,True),(1,False));command('host-shortcuts',close=True);time.sleep(.35)
 tabs=command('tab-status')['tabs']
 assert any(x['title']=='Terminal · '+os.uname().nodename for x in tabs),tabs
 assert any(x.get('colour_icon') and '#34d399' in x.get('svg','') for x in tabs),tabs
 command('focus-view');time.sleep(.2);eventually(lambda:command('inspect')['active']);keys((100,True),(100,False));time.sleep(.2)
 keys((56,True),(50,True),(50,False),(56,False));eventually(lambda:command('sidebar-shell')['shown'])
 command('sidebar-shell',shown=False);command('focus-view')
 keys((56,True),(30,True),(30,False),(56,False));eventually(lambda:'Saved agent hosts' in command('host-shortcuts')['sections'])
 keys((1,True),(1,False));command('host-shortcuts',close=True);time.sleep(.35);command('focus-view');keys((100,True),(100,False))
 checks.append('real-GDK-Alt-M-and-Alt-A-work-with-collapsed-actions-and-Map-keys-host-title-colour-status')

 command('focus-view');time.sleep(.5)
 before_draw=command('inspect')['drawn'][0]
 with Client(sock)as c:
  extra=c.call('session.create',{'name':'generated-idle-neighbor','argv':['/bin/bash','--noprofile','--norc']})
  c.call('pane.sendText',{'pane':extra['pane'],'text':"for i in 1 2 3 4 5; do printf 'neighbor update\\n'; sleep .15; done\n"})
  time.sleep(1.5)
 assert command('inspect')['drawn'][0]==before_draw,'unrelated session redrew idle canvas'
 checks.append('idle-selected-canvas-does-not-redraw-for-other-session-output')
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
 command('focus-view',view=0)
 before_height=command('inspect')['cell_metrics'][1]
 command('chrome',line_height=.95);eventually(lambda:command('inspect')['cell_metrics'][1]<before_height)
 command('chrome',line_height=1.0)
 target=command('inspect')['target']
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
