#!/usr/bin/env python3
"""Private GTK/Secret Service and authenticated loopback input; never real GDM."""
import argparse,hashlib,json,os,subprocess,tempfile,time
from pathlib import Path
from spike import Client,eventually
p=argparse.ArgumentParser();p.add_argument('--binary',required=True);p.add_argument('--gui',required=True);p.add_argument('--server',required=True);p.add_argument('--evidence',type=Path,required=True);a=p.parse_args()
runtime=Path(os.environ['XDG_RUNTIME_DIR']);assert runtime.name=='display' and runtime.parent.parent.name=='results' and os.environ['WAYLAND_DISPLAY'].startswith('ruwac-')
root=Path(tempfile.mkdtemp(prefix='dual-login-',dir=a.evidence));root.chmod(0o700);env={**os.environ,'GTK_A11Y':'none','GSETTINGS_BACKEND':'memory'}
for key,name in [('HOME','home'),('XDG_CONFIG_HOME','config'),('XDG_DATA_HOME','data'),('XDG_CACHE_HOME','cache')]:env[key]=str(root/name);Path(env[key]).mkdir(mode=0o700)
folder=Path(env['XDG_CONFIG_HOME'])/'RUWAC';folder.mkdir(mode=0o700);store=folder/'machines.json';sock=root/'control.sock';children=[];seq=0;results=[];bus=None

def start(argv,name):
 log=(root/(name+'.log')).open('wb');p=subprocess.Popen(argv,env=env,stdout=log,stderr=log);children.append((p,log));return p

def command(action,**kw):
 global seq
 seq+=1;tmp=root/'request.tmp';tmp.write_text(json.dumps({'seq':seq,'action':action,**kw}));tmp.replace(root/'gui-request.json')
 def read():
  try:d=json.loads((root/'gui-response.json').read_text());return d['result']if d['seq']==seq else None
  except (OSError,ValueError):return None
 return eventually(read,10)

def keyring(reference,value=None,verb='lookup'):
 args=['secret-tool',verb]
 if verb=='store':args+=['--label=RUWAC generated disposable login test']
 args+=['application','RUWAC','profile',reference]
 return subprocess.run(args,input=value,env=env,stdout=subprocess.PIPE,stderr=subprocess.DEVNULL,timeout=10)

def receipt(port):
 try:return json.loads((root/f'input-{port}.json').read_text())
 except (OSError,ValueError):return {'units':0}

def focus():
 subprocess.run(['xdotool','mousemove','350','250','click','1'],env={**env,'DISPLAY':env['RUWAC_XTEST_DISPLAY']},check=True)
 eventually(lambda:command('inspect')['active'])

def editor(**kw):return command('profile-editor',**kw)
try:
 lines=subprocess.check_output(['dbus-daemon','--session','--fork','--print-address=1','--print-pid=1'],env=env,text=True).splitlines();bus=int(lines[1]);env['DBUS_SESSION_BUS_ADDRESS']=lines[0]
 control=root/'keyring';control.mkdir(mode=0o700)
 log=(root/'keyring.log').open('wb');provider=subprocess.Popen(['gnome-keyring-daemon','--foreground','--components=secrets','--control-directory='+str(control),'--unlock'],env=env,stdin=subprocess.PIPE,stdout=log,stderr=log);children.append((provider,log));provider.stdin.write(b'generated-private-keyring-only\n');provider.stdin.close();time.sleep(1)
 assert keyring('service-test',b'generated-local-only','store').returncode==0
 subprocess.run(['openssl','req','-x509','-newkey','rsa:2048','-nodes','-days','1','-subj','/CN=RUWAC-loopback','-addext','subjectAltName=IP:127.0.0.1','-keyout',str(root/'key.pem'),'-out',str(root/'cert.pem')],check=True,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL);(root/'key.pem').chmod(0o600)
 start([a.server,str(root)],'server');eventually(lambda:(root/'server-ready.json').exists());ports=json.loads((root/'server-ready.json').read_text())['ports'];port=ports[0]
 store.write_text(json.dumps({'schema':1,'items':[],'groups':[]}));store.chmod(0o600)
 daemon=start([a.binary,'--socket',str(sock),'serve'],'daemon');eventually(lambda:sock.exists())
 with Client(sock)as c:session=c.call('session.create',{'name':'Dual credential test','argv':['/bin/cat']})
 gui=start([a.gui,'--show-machines','--socket',str(sock),'--session',session['session'],'--fixture-dir',str(root)],'gui');eventually(lambda:command('inspect'),15);focus()
 editor(fields={'0':'Fedora dual','3':f'127.0.0.1:{port}','4':'fixture','9':'service-test','10':str(root/'cert.pem'),'26':1},named={'gnome-username':'linux-fixture','gnome-secret-reference':'gnome-test'},response='save')
 row=json.loads(store.read_text())['items'][0];assert row['username']=='fixture'and row['secret_reference']=='service-test'and row['preferences']['gnome_username']=='linux-fixture';original_id=row['id']
 assert 'password'not in store.read_text();results.append({'id':'DUAL-GTK-SQL-JSON-INDEPENDENT-USERNAME-REF','status':'PASS'})
 editor(operation='edit',named={'gnome-password':'generated-second-stage-only'},credential_action='Set / replace GNOME password',response='keep');eventually(lambda:keyring('gnome-test').stdout.rstrip(b'\n')==b'generated-second-stage-only');editor(operation='apply',response='cancel')
 assert keyring('service-test').stdout.rstrip(b'\n')==b'generated-local-only'
 editor(operation='edit',named={'gnome-password':'generated-replacement-only'},credential_action='Set / replace GNOME password',response='keep');eventually(lambda:keyring('gnome-test').stdout.rstrip(b'\n')==b'generated-replacement-only');editor(operation='apply',response='cancel')
 results.append({'id':'DUAL-GTK-SET-REPLACE-SECRET-SERVICE-SEPARATION','status':'PASS'})
 editor(operation='edit',named={'rdp-password':'generated-service-replacement'},credential_action='Set / replace credential',response='keep');eventually(lambda:keyring('service-test').stdout.rstrip(b'\n')==b'generated-service-replacement');editor(operation='apply',response='cancel');assert keyring('gnome-test').stdout.rstrip(b'\n')==b'generated-replacement-only'
 editor(operation='edit',credential_action='Forget credential',response='keep');eventually(lambda:keyring('service-test').returncode!=0);editor(operation='apply',response='cancel');assert keyring('gnome-test').stdout.rstrip(b'\n')==b'generated-replacement-only'
 editor(operation='edit',named={'rdp-password':'generated-local-only'},credential_action='Set / replace credential',response='keep');eventually(lambda:keyring('service-test').stdout.rstrip(b'\n')==b'generated-local-only');editor(operation='apply',response='cancel')
 editor(operation='edit',fields={'9':'gnome-test'},credential_action='Forget credential',response='keep');time.sleep(.2);assert keyring('gnome-test').stdout.rstrip(b'\n')==b'generated-replacement-only';editor(operation='apply',response='cancel')
 results.append({'id':'DUAL-SERVICE-SET-REPLACE-FORGET-AND-CROSS-STAGE-GUARD','status':'PASS'})
 command('profiles',search='Fedora dual');time.sleep(.4);command('profiles',open=True);eventually(lambda:command('rdp-inspect',rdp=0)['size']==[320,180],20);focus()
 assert command('rdp-assist')['action_grid_row']==4 and command('rdp-assist')['action_visible']
 assert next(c for c in command('rdp-toolbar')['controls']if c['action']=='gnome-sign-in')['mapped']
 assert not command('rdp-assist',operation='stale-frame')['dialog_shown'];assert receipt(port)['units']==0
 results.append({'id':'DUAL-STALE-RENDERED-GENERATION-REFUSES-ASSISTANCE','status':'PASS','scope':'Injected generation mismatch on a real decoded/drawn loopback framebuffer'})
 command('rdp-assist',operation='open');assert command('rdp-assist')['dialog_shown'];command('rdp-assist',field='username',confirmed=False);time.sleep(.4);assert receipt(port)['units']==0
 command('rdp-assist',operation='open');assert command('rdp-assist')['dialog_shown'];command('rdp-assist',field='username',confirmed=True);eventually(lambda:receipt(port)['units']==len('linux-fixture'),10)
 command('rdp-assist',operation='open');command('rdp-assist',field='password',confirmed=True);expected='linux-fixturegenerated-replacement-only';eventually(lambda:receipt(port).get('sha256')==hashlib.sha256(expected.encode('utf-16-le')).hexdigest(),10)
 results.append({'id':'DUAL-CONFIRMED-FIELD-AUTHENTICATED-RDP-INPUT','status':'PASS','scope':'Actual loopback CredSSP/framebuffer/Unicode channel, operator confirmation through synthetic GTK callback; no GDM login claim'})
 before=receipt(port);command('rdp-assist',operation='open');command('rdp-toolbar',control='reconnect');command('rdp-assist',field='password',confirmed=True);eventually(lambda:command('rdp-inspect',rdp=0)['size']==[320,180],20);time.sleep(.4);assert receipt(port)==before
 results.append({'id':'DUAL-RECONNECT-INVALIDATES-SCREEN-CONFIRMATION','status':'PASS'})
 focus();command('rdp-assist',operation='open');command('rdp-assist',field='password',confirmed=True);command('sidebar-shell',shown=True);time.sleep(.5);assert receipt(port)==before;command('sidebar-shell',shown=False);time.sleep(.4)
 results.append({'id':'DUAL-OVERLAY-FOCUS-CHANGE-CANCELS-SENSITIVE-INPUT','status':'PASS'})
 before=receipt(port);command('rdp-assist',operation='open');command('switch');command('rdp-assist',field='password',confirmed=True);time.sleep(.5);assert receipt(port)==before
 results.append({'id':'DUAL-TAB-SWITCH-CANCELS-SENSITIVE-INPUT','status':'PASS'})
 # Independent Forget uses actual editor controls, transport reference remains usable.
 editor(operation='edit',credential_action='Forget GNOME password',response='keep');eventually(lambda:keyring('gnome-test').returncode!=0);editor(operation='apply',response='cancel');assert keyring('service-test').stdout.rstrip(b'\n')==b'generated-local-only'
 results.append({'id':'DUAL-GTK-FORGET-DOES-NOT-DELETE-SERVICE','status':'PASS'})
 command('rdp-switch',rdp=0);focus();before=receipt(port);command('rdp-assist',operation='open');command('rdp-assist',field='password',confirmed=True);eventually(lambda:'missing' in command('rdp-assist')['status'].lower() or 'locked' in command('rdp-assist')['status'].lower(),10);assert receipt(port)==before
 results.append({'id':'DUAL-MISSING-KEYRING-REF-NO-INPUT-FALLBACK','status':'PASS'})
 editor(operation='edit',named={'gnome-username':'changed-linux-user'},response='save');focus();assert not command('rdp-assist',operation='open')['dialog_shown'];assert receipt(port)==before
 results.append({'id':'DUAL-STALE-SAVED-PROFILE-REFUSES-ASSISTANCE','status':'PASS'})
 assert keyring('service-test',b'generated-invalid-transport','store').returncode==0
 editor(operation='edit',fields={'3':f'127.0.0.1:{ports[1]}'},named={'gnome-auto-login':True},response='save')
 command('profiles',search='Fedora dual');time.sleep(.4);command('profiles',open=True)
 def wrong_service_rejected():
  q=command('rdp-inspect',rdp=1)
  reply=subprocess.run([a.binary,'--socket',str(sock),'rdp','list','--json'],env=env,capture_output=True,text=True,timeout=8);assert reply.returncode==0,reply.stderr
  row=next(v for v in json.loads(reply.stdout)['result']['views']if v['transport_id']==q['transport_id'])
  if row['phase']=='error'and row['authentication_error']:
   # Worker phase and GTK status are independent asynchronous observations.
   # Wait for the bounded native update rather than asserting an older label.
   if q['status'].startswith('RDP service authentication failed') and q['gnome_auto']['step']=='Stopped':return q
   return None
  return None
 rejected=eventually(wrong_service_rejected,15);assert rejected['size']==[0,0]
 editor(operation='edit',named={'gnome-auto-login':False},response='save')
 assert keyring('service-test',b'generated-local-only','store').returncode==0
 results.append({'id':'DUAL-WRONG-SERVICE-PASSWORD-NO-DESKTOP','status':'PASS'})
 # Restarting a named additional window shares daemon/profile DB, saves layout separately.
 other=root/'second-window';other.mkdir(mode=0o700);window_id='window_'+'a'*32
 unavailable=root/'unavailable-helper';unavailable.mkdir(mode=0o700);helper=unavailable/'secret-tool';helper.write_text('#!/bin/sh\nDBUS_SESSION_BUS_ADDRESS="unix:path=$RUWAC_UNAVAILABLE_BUS" exec /usr/bin/secret-tool "$@"\n');helper.chmod(0o700)
 original_path=env['PATH'];env['PATH']=str(unavailable)+':'+original_path;env['RUWAC_UNAVAILABLE_BUS']=str(root/'absent-secret-service.sock')
 second=start([a.gui,'--new-instance','--window-id',window_id,'--socket',str(sock),'--fixture-dir',str(other)],'second-window');env['PATH']=original_path
 other_seq=0
 def second_command(action,**kw):
  global other_seq
  other_seq+=1;tmp=other/'request.tmp';tmp.write_text(json.dumps({'seq':other_seq,'action':action,**kw}));tmp.replace(other/'gui-request.json')
  def reply():
   try:d=json.loads((other/'gui-response.json').read_text());return d['result']if d['seq']==other_seq else None
   except(OSError,ValueError):return None
  return eventually(reply,10)
 state=second_command('inspect');assert state['tabs']==1 and gui.poll()is None
 with Client(sock)as c:assert c.call('health')['panes']>=2
 before_metadata=store.read_bytes();second_command('profile-editor',operation='edit',named={'gnome-password':'generated-unavailable-service'},credential_action='Set / replace GNOME password',response='keep')
 eventually(lambda:'unavailable' in(second_command('profile-editor',operation='inspect',response='keep').get('credential_status')or'').lower(),10);assert store.read_bytes()==before_metadata;second_command('profile-editor',operation='inspect',response='cancel')
 results.append({'id':'DUAL-UNAVAILABLE-SECRET-SERVICE-NATIVE-EDITOR-NO-METADATA-PASSWORD','status':'PASS'})
 second_command('window-close');eventually(lambda:second.poll()is not None)
 assert gui.poll()is None and daemon.poll()is None;assert list((folder/'windows'/window_id).glob('workspace*.json'))
 results.append({'id':'MULTIWINDOW-INDEPENDENT-LAYOUT-SHARED-DAEMON-CLOSE','status':'PASS'})
 assert json.loads(store.read_text())['items'][0]['id']==original_id
finally:
 for process,log in reversed(children):
  if process.poll()is None:
   process.terminate()
   try:process.wait(timeout=8)
   except subprocess.TimeoutExpired:process.kill();process.wait()
  log.close()
 if bus:
  try:os.kill(bus,15)
  except ProcessLookupError:pass
print(json.dumps({'results':results,'sha256':{n:hashlib.sha256(Path(v).read_bytes()).hexdigest()for n,v in [('ruwac',a.binary),('ruwac-desktop',a.gui)]},'scope':'Owned Wayland/Xvfb and disposable Secret Service, authenticated loopback, no owner secrets or real GDM acceptance'},indent=2))
