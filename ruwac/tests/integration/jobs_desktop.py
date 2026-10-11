#!/usr/bin/env python3
"""Actual owned GTK process monitor, search and exact stable-pane jump."""
import argparse,hashlib,json,os,subprocess,tempfile,time
from pathlib import Path
from spike import Client,eventually
p=argparse.ArgumentParser();p.add_argument('--binary',required=True);p.add_argument('--gui',required=True);p.add_argument('--evidence',type=Path,required=True);a=p.parse_args()
root=Path(tempfile.mkdtemp(prefix='jobs-desktop-',dir=a.evidence));root.chmod(0o700);env={**os.environ,'GTK_A11Y':'none','GSETTINGS_BACKEND':'memory'}
for key,name in [('HOME','home'),('XDG_CONFIG_HOME','config'),('XDG_DATA_HOME','data'),('XDG_CACHE_HOME','cache')]:
 env[key]=str(root/name);Path(env[key]).mkdir(mode=0o700)
sock=root/'control.sock';gui=None;daemon=None;notification=None;seq=0;results=[]
def rpc(method,params=None):
 with Client(sock) as c:return c.call(method,params)
def command(action,**kw):
 global seq
 seq+=1;temp=root/'request.tmp';temp.write_text(json.dumps({'seq':seq,'action':action,**kw}));temp.replace(root/'gui-request.json')
 def read():
  try:d=json.loads((root/'gui-response.json').read_text());return d['result'] if d['seq']==seq else None
  except (OSError,ValueError):return None
 return eventually(read,10)
try:
 notification=subprocess.Popen(['python3',str(Path(__file__).parents[1]/'fixtures/notifications.py'),str(root)],env=env,stdout=(root/'notifications.log').open('wb'),stderr=subprocess.STDOUT)
 eventually(lambda:(root/'notification-ready').exists())
 daemon=subprocess.Popen([a.binary,'--socket',str(sock),'serve'],env=env,stdout=(root/'daemon.log').open('wb'),stderr=subprocess.STDOUT);eventually(lambda:sock.exists())
 s=rpc('session.create',{'name':'Same name','argv':['/bin/sh','-c','sleep 300']})
 second=rpc('session.create',{'name':'Same name','argv':['/bin/sh','-c','sleep 300']})
 window=rpc('window.create',{'session':second['session'],'name':'Jump target','argv':['/bin/sh','-c','sleep 300']})
 rpc('window.select',{'window':second['window']})
 gui=subprocess.Popen([a.gui,'--socket',str(sock),'--session',s['session'],'--fixture-dir',str(root)],env=env,stdout=(root/'gui.log').open('wb'),stderr=subprocess.STDOUT)
 eventually(lambda:command('inspect'),15);command('jobs',operation='open')
 state=eventually(lambda:(v if len((v:=command('jobs'))['rows'])==3 else None),10)
 assert all('Running' in r['summary'] for r in state['rows']),state
 assert {r['pane'] for r in state['rows']}=={s['pane'],second['pane'],window['pane']}
 time.sleep(1);assert all('Running' in r['summary'] for r in command('jobs')['rows'])
 results.append({'id':'JOBS-NATIVE-LIVE-QUIET','status':'PASS','native_rows':3,'quiet_not_inferred_finished':True})
 command('jobs',query='Jump target');state=eventually(lambda:(v if len((v:=command('jobs'))['rows'])==1 else None))
 assert state['rows'][0]['pane']==window['pane'],state
 results.append({'id':'JOBS-NATIVE-SEARCH','status':'PASS'})
 before=rpc('job.query',{'pane':window['pane']})['pid'];command('jobs',jump=window['pane'])
 eventually(lambda:command('jobs')['open'] is False)
 state=eventually(lambda:(v if (v:=command('inspect',view=1))['tabs']==2 and v['target']==window['pane'] else None),10)
 assert rpc('job.query',{'pane':window['pane']})['pid']==before
 assert next(s for s in rpc('session.list') if s['id']==second['session'])['active_window']==window['window']
 results.append({'id':'JOBS-NATIVE-EXACT-JUMP','status':'PASS','duplicate_names':True,'same_pid':True,'selected_window_and_pane':True})
 command('jobs',operation='open');eventually(lambda:len(command('jobs')['rows'])==3)
 quiet=rpc('session.create',{'name':'Exit notification','argv':['/bin/sh','-c','read answer; exit 7','PRIVATE-NOTIFICATION-ARG']})
 eventually(lambda:any(r['pane']==quiet['pane'] for r in command('jobs')['rows']))
 assert not (root/'notifications.json').exists()
 command('jobs',notify=True);time.sleep(.7);rpc('pane.sendText',{'pane':quiet['pane'],'text':'done\n'})
 eventually(lambda:(root/'notifications.json').exists(),10);messages=json.loads((root/'notifications.json').read_text())
 assert len(messages)==1 and messages[0]['summary']=='Terminal job exited' and 'Exited (7)' in messages[0]['body'],messages
 assert 'PRIVATE-NOTIFICATION-ARG' not in json.dumps(messages) and str(root) not in json.dumps(messages)
 time.sleep(1);assert len(json.loads((root/'notifications.json').read_text()))==1
 results.append({'id':'JOBS-NATIVE-OPTIN-NOTIFICATION','status':'PASS','actual_private_dbus_delivery':True,'one_observed_exit':True,'quiet_no_notification':True})
 command('jobs',close=True);eventually(lambda:command('jobs')['retained_search_widgets']==0)
 for _ in range(5):
  command('jobs',operation='open');eventually(lambda:command('jobs')['retained_search_widgets']==1)
  command('jobs',operation='open');assert command('jobs')['retained_search_widgets']==1
  command('jobs',close=True);eventually(lambda:command('jobs')['retained_search_widgets']==0)
 results.append({'id':'JOBS-NATIVE-DIALOG-CLEANUP','status':'PASS','real_widget_finalization':True,'repeated_open_close':5,'existing_dialog_reused':True})
finally:
 if gui is not None and gui.poll() is None:gui.terminate();gui.wait(timeout=10)
 if daemon is not None and daemon.poll() is None:
  try:rpc('daemon.stop');daemon.wait(timeout=8)
  except Exception:daemon.kill();daemon.wait()
 if notification is not None and notification.poll() is None:notification.terminate();notification.wait(timeout=5)
 receipt={'results':results,'hashes':{n:hashlib.sha256(Path(path).read_bytes()).hexdigest() for n,path in [('ruwac',a.binary),('ruwac-desktop',a.gui)]},'scope':'Real GTK widgets and notification delivery on private compositor/D-Bus; synthetic actions, not physical GNOME acceptance'}
 (root/'receipt.json').write_text(json.dumps(receipt,indent=2)+'\n');print(json.dumps(receipt,indent=2),flush=True)
assert len(results)==5
