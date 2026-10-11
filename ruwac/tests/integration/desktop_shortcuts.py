#!/usr/bin/env python3
"""Global GTK tab shortcuts via actual nested-compositor XTest events.
RDP rendering in this test is synthetic; no remote authentication is claimed.
"""
import argparse,importlib.util,json,os,subprocess,sys,tempfile,time
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('--binary',required=True);p.add_argument('--gui',required=True);p.add_argument('--evidence',type=Path,required=True);a=p.parse_args()
spec=importlib.util.spec_from_file_location('spike',Path(__file__).with_name('spike.py'));spike=importlib.util.module_from_spec(spec);spec.loader.exec_module(spike)
root=Path(tempfile.mkdtemp(prefix='desktop-shortcuts-',dir=a.evidence));root.chmod(0o700);env={**os.environ,'GTK_A11Y':'none','GSETTINGS_BACKEND':'memory'}
for k,d in [('HOME','home'),('XDG_CONFIG_HOME','config'),('XDG_DATA_HOME','data'),('XDG_CACHE_HOME','cache')]:env[k]=str(root/d);Path(env[k]).mkdir(mode=0o700)
sock=root/'daemon.sock';seq=0;gui=None;cases=[]
def command(action,**kw):
 global seq
 seq+=1;t=root/'gui-request.tmp';t.write_text(json.dumps({'seq':seq,'action':action,**kw}));t.replace(root/'gui-request.json')
 def read():
  try:r=json.loads((root/'gui-response.json').read_text());return r['result'] if r['seq']==seq else None
  except (FileNotFoundError,ValueError):return None
 return spike.eventually(read,10)
def key(chord):
 subprocess.run(['xdotool','key','--clearmodifiers',chord],env={**env,'DISPLAY':os.environ['RUWAC_XTEST_DISPLAY']},check=True);time.sleep(.15)
def selected(index):return spike.eventually(lambda:command('inspect')['selected_tab']==index,5)
try:
 subprocess.run([a.binary,'--socket',str(sock),'start'],check=True,capture_output=True)
 ids=[]
 for i in range(8):
  demo="import sys;print('RUWAC  |  Native machine workspace\\n\\nPersistent terminals · SSH · Embedded RDP · Files\\n\\nAlt+1…9 / Alt+0  switch workspace tabs\\nCtrl+A           Screen navigation\\nCtrl+Shift+C/V   desktop copy / paste\\n\\n\\x1b[32mReady\\x1b[0m — programs survive closing the desktop',flush=True);sys.stdin.buffer.read()"
  with spike.Client(sock) as client:ids.append(client.call('session.create',{'name':f'Workspace {i+1}','argv':[sys.executable,'-u','-c',demo]})['session'])
 argv=[a.gui,'--socket',str(sock),'--fixture-dir',str(root)]
 for sid in ids:argv+=['--session',sid]
 gui=subprocess.Popen(argv,env=env,stdout=(root/'gui.log').open('wb'),stderr=subprocess.STDOUT)
 state=spike.eventually(lambda: (r if r.get('active') and all(n==1 for n in r['panes']) else None) if (r:=command('inspect')) else None,10);assert state['active'] and state['dark'] and state['display']=='GdkWaylandDisplay',state
 key('alt+0');assert command('inspect')['selected_tab']==state['selected_tab']
 cases.append({'id':'GTK-ALT-MISSING-TAB','status':'PASS','no_tab_created':command('inspect')['tabs']==8})
 command('rdp-open');command('files-open',profile={'id':'fixture','name':'Files','group':'Fixture','protocol':'SSH','server':'127.0.0.1:1','username':'fixture','secret_reference':'fixture','favorite':False,'last_used':0,'tags':[]},path='/');assert command('inspect')['tabs']==10
 for digit,index in [('1',0),('2',1),('3',2),('4',3),('5',4),('6',5),('7',6),('8',7),('9',8),('0',9),('1',0)]:key('alt+'+digit);selected(index)
 cases.append({'id':'GTK-ALT-ALL-TABS','status':'PASS','terminal_tabs':8,'rdp_tabs':1,'file_tabs':1,'input':'XTest through nested Weston to real GTK capture controller','rdp_authentication':'not tested; synthetic frame'})
 for chord in ['ctrl+alt+2','shift+alt+2','2']:key(chord);assert command('inspect')['selected_tab']==0,chord
 key('Caps_Lock');key('alt+2');selected(1);key('Caps_Lock');key('alt+1');selected(0)
 cases.append({'id':'GTK-ALT-MODIFIER-GUARDS','status':'PASS','CtrlAlt_and_ShiftAlt_untouched':True,'CapsLock_works':True,'physical_AltGr':'not qualified'})
 for index in range(9,2,-1):command('close-tab',tab=index)
 assert command('inspect')['tabs']==3;key('alt+3');selected(2);time.sleep(.3);command('snapshot');spike.eventually(lambda:(root/'native-rust.png').exists());cases.append({'id':'GTK-DARK-UI','status':'PASS','actual_native_render':True,'desktop_display':'nested Wayland'})
 for argv in [[a.gui,'--theme','invalid'],[a.gui,'--theme']]:assert subprocess.run(argv,capture_output=True).returncode!=0
finally:
 if gui is not None and gui.poll() is None:gui.terminate();gui.wait(timeout=10)
 if sock.exists():subprocess.run([a.binary,'--socket',str(sock),'rpc','daemon.stop'],capture_output=True,timeout=8)
 receipt={'cases':cases,'evidence':str(root)};(root/'receipt.json').write_text(json.dumps(receipt,indent=2)+'\n');(a.evidence/'desktop-shortcuts-acceptance.json').write_text(json.dumps(receipt,indent=2)+'\n');print(json.dumps(receipt,indent=2))
