#!/usr/bin/env python3
"""Real owned GTK lifecycle and PTY identity checks; RDP pixels are synthetic."""
import argparse,json,os,socket,struct,subprocess,tempfile,time,uuid
from pathlib import Path
from spike import Client,eventually,exact
p=argparse.ArgumentParser();p.add_argument('--binary',required=True);p.add_argument('--gui',required=True);p.add_argument('--server',required=True);p.add_argument('--evidence',type=Path,required=True);a=p.parse_args()
root=Path(tempfile.mkdtemp(prefix='workspace-lifecycle-',dir=a.evidence));root.chmod(0o700)
env={**os.environ,'RUWAC_GUI_ENTRY':a.gui,'GTK_A11Y':'none','GSETTINGS_BACKEND':'memory'}
for key,name in [('XDG_CONFIG_HOME','config'),('XDG_DATA_HOME','data'),('XDG_CACHE_HOME','cache')]:env[key]=str(root/name);Path(env[key]).mkdir(mode=0o700)
sock=root/'control.sock';ipc=sock.with_suffix('.gui.sock');seq=0;gui=None;server=None;cases=[]
def api(args,ok=True):
 r=subprocess.run([a.binary,'--socket',str(sock),'workspace',*args,'--json'],env=env,capture_output=True,text=True,timeout=7);assert(r.returncode==0)==ok,(args,r.stdout,r.stderr);return json.loads(r.stdout)
def raw(method,params,key=None):
 with Client(ipc)as c:
  req={'version':0,'id':uuid.uuid4().hex,'method':method,'params':params}
  if key:req['idempotency_key']=key
  b=json.dumps(req).encode();c.s.sendall(struct.pack('!I',len(b))+b);return json.loads(exact(c.s,struct.unpack('!I',exact(c.s,4))[0]))
def command(action,**kw):
 global seq
 seq+=1;t=root/'gui-request.tmp';t.write_text(json.dumps({'seq':seq,'action':action,**kw}));t.replace(root/'gui-request.json')
 def read():
  try:r=json.loads((root/'gui-response.json').read_text());return r['result']if r['seq']==seq else None
  except(OSError,ValueError):return None
 return eventually(read,10)
def listing():return api(['list'])['result']
def identities():return {v['id']for v in listing()['views']}
def pids():
 with Client(sock)as c:return {s['id']:[p['pid']for p in c.call('pane.list',{'session':s['id']})]for s in c.call('session.list')}

def subscribe(types):return api(['events','subscribe','--types',types])['result']
def matching(s,predicate):
 v=poll(s);return v if any(predicate(e)for e in v['events'])else None
def poll(s,cursor=None,limit=50):return api(['events','poll','--subscription',s['subscription'],'--after',str(s['cursor']if cursor is None else cursor),'--limit',str(limit)])['result']
try:
 subprocess.run(['openssl','req','-x509','-newkey','rsa:2048','-nodes','-days','1','-subj','/CN=RUWAC-local-fixture','-addext','subjectAltName=IP:127.0.0.1','-keyout',str(root/'key.pem'),'-out',str(root/'cert.pem')],check=True,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL);(root/'key.pem').chmod(0o600)
 server=subprocess.Popen([a.server,str(root)],env=env,stdout=(root/'server.log').open('wb'),stderr=subprocess.STDOUT);eventually(lambda:(root/'server-ready.json').exists(),10);port=json.loads((root/'server-ready.json').read_text())['ports'][0]
 gui=subprocess.Popen([a.binary,'--socket',str(sock),'gui','--fixture-dir',str(root)],env=env,stdout=(root/'gui.log').open('wb'),stderr=subprocess.STDOUT);command('inspect');eventually(lambda:ipc.exists()and listing(),10);time.sleep(.3)
 descriptor=api(['describe'])['result'];assert 'workspace.events.subscribe'in descriptor['methods'] and not descriptor['permissions']['focus_control']
 all_events=subscribe('tab,focus,layout,rdp');tab_events=subscribe('tab');assert all_events['delivery']=='poll'and all_events['capacity']==256
 baseline=pids();command('new-terminal');eventually(lambda:len(listing()['views'])==2);all_pids=pids()
 first=eventually(lambda:matching(all_events,lambda e:e['kind']=='tab'and e['data']['change']=='opened'))
 assert first['instance']==all_events['instance']and all(e['cursor']>all_events['cursor']for e in first['events'])
 filtered=poll(tab_events);assert filtered['events']and all(e['kind']=='tab'for e in filtered['events'])
 # Explicit metadata observation never grants focus/lifecycle control.
 target=listing()['tabs'][0]['id'];assert api(['activate','--id',target],False)['error']['code']=='PERMISSION_DENIED'
 cases.append({'id':'NATIVE_EVENT_SUBSCRIBE_FILTER_CURSOR_PERMISSION','PASS':True})
 join=subscribe('layout,tab');command('mixed-join',tab=0,vertical=True);time.sleep(.3);changed=poll(join);assert any(e['kind']=='layout'for e in changed['events'])and any(e['kind']=='tab'and e['data']['change']=='closed'for e in changed['events']);assert pids()==all_pids
 cases.append({'id':'NATIVE_LAYOUT_AND_TAB_EVENTS_PRESERVE_PIDS','PASS':True})
 rdp=subscribe('rdp');command('rdp-open',port=port)
 eventually(lambda:command('rdp-inspect',rdp=0)['size']==[320,180],20)
 connected=eventually(lambda:matching(rdp,lambda e:e['kind']=='rdp'and e['data']['phase']=='connected'and e['data']['frame_available']),10)
 command('close-rdp',rdp=0);time.sleep(.3);removed=poll(rdp,connected['cursor']);assert any(e['data']['phase']=='removed'for e in removed['events'])
 assert all(marker not in json.dumps(connected)for marker in ['secret_reference','password','clipboard','127.0.0.1','generated-local-only','cert.pem'])
 cases.append({'id':'AUTHENTICATED_LOOPBACK_RDP_STATE_AND_REMOVAL_EVENTS','PASS':True})
 assert raw('workspace.events.subscribe',{'types':['clipboard']})['error']['code']=='INVALID_REQUEST'
 assert raw('workspace.events.poll',{'subscription':all_events['subscription'],'after':0,'limit':51})['error']['code']=='INVALID_REQUEST'
 assert raw('workspace.events.subscribe',{'types':['tab'],'password':'EVENT_AUDIT_SENTINEL'})['error']['code']=='INVALID_REQUEST'
 assert raw('workspace.events.subscribe',{'types':['tab']},key='stale-events')['error']['code']=='INVALID_REQUEST'
 assert 'EVENT_AUDIT_SENTINEL'not in(root/'gui.log').read_text()
 assert api(['events','unsubscribe','--subscription',all_events['subscription']])['result']['unsubscribed']
 assert api(['events','poll','--subscription',all_events['subscription'],'--after','0','--limit','1'],False)['error']['code']=='STALE_SUBSCRIPTION'
 cases.append({'id':'EVENT_UNSUBSCRIBE_STRICT_PARAMETERS_AND_AUDIT_PRIVACY','PASS':True})
 command('workspace-close');gui.wait(timeout=10);assert pids()==all_pids
finally:
 if gui is not None and gui.poll()is None:gui.terminate();gui.wait(timeout=10)
 if server is not None and server.poll()is None:server.terminate();server.wait(timeout=10)
 if sock.exists():subprocess.run([a.binary,'--socket',str(sock),'rpc','daemon.stop'],env=env,capture_output=True,timeout=8)
 result={'cases':cases,'scope':'Actual GTK/PTY and authenticated loopback RDP with generated framebuffer; synthetic local GTK input, no external or physical desktop claim','evidence':str(root)};(root/'receipt.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result,indent=2))
