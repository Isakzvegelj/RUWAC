#!/usr/bin/env python3
"""Real owned GTK lifecycle and PTY identity checks; RDP pixels are synthetic."""
import argparse,json,os,socket,struct,subprocess,tempfile,time,uuid
from pathlib import Path
from spike import Client,eventually,exact
p=argparse.ArgumentParser();p.add_argument('--binary',required=True);p.add_argument('--gui',required=True);p.add_argument('--evidence',type=Path,required=True);a=p.parse_args()
root=Path(tempfile.mkdtemp(prefix='workspace-lifecycle-',dir=a.evidence));root.chmod(0o700)
env={**os.environ,'RUWAC_GUI_ENTRY':a.gui,'GTK_A11Y':'none','GSETTINGS_BACKEND':'memory'}
for key,name in [('XDG_CONFIG_HOME','config'),('XDG_DATA_HOME','data'),('XDG_CACHE_HOME','cache')]:env[key]=str(root/name);Path(env[key]).mkdir(mode=0o700)
sock=root/'control.sock';ipc=sock.with_suffix('.gui.sock');seq=0;gui=None;cases=[]
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
try:
 gui=subprocess.Popen([a.binary,'--socket',str(sock),'gui','--fixture-dir',str(root)],env=env,stdout=(root/'gui.log').open('wb'),stderr=subprocess.STDOUT);command('inspect');eventually(lambda:ipc.exists()and listing(),10)
 assert api(['saved'])['result']['saved_workspaces']==[]
 assert api(['save','--name','Mixed'],False)['error']['code']=='PERMISSION_DENIED'
 command('workspace-permission',enabled=True)
 command('new-terminal');eventually(lambda:len(listing()['views'])==2)
 command('rdp-open');command('files-open',profile={'id':'fixture','name':'Files','group':'Fixture','protocol':'SSH','server':'127.0.0.1:1','username':'fixture','secret_reference':'private-reference','favorite':False,'last_used':0,'tags':[]},path='/')
 command('tab-select',tab=0);command('mixed-join',tab=2,vertical=True);command('mixed-join',tab=2,vertical=False)
 original=listing();original_ids=identities();original_pids=pids();assert len(original_ids)==4
 first=raw('workspace.save',{'name':'Mixed private workspace'},key='saved-once');assert 'result'in first,first
 snapshot=first['result']['id'];second=raw('workspace.save',{'name':'Mixed private workspace'},key='saved-once');assert second['result']['id']==snapshot
 assert raw('workspace.save',{'name':'Other'},key='saved-once')['error']['code']=='IDEMPOTENCY_CONFLICT'
 summaries=api(['saved'])['result'];assert len(summaries['saved_workspaces'])==1;assert all(marker not in json.dumps(summaries)for marker in ['private-reference','127.0.0.1','secret_reference'])
 files=list((root/'config/RUWAC/snapshots').glob('workspace_*/*.json'));assert len(files)==1 and files[0].stat().st_mode&0o777==0o600
 cases.append({'id':'SNAPSHOT_PRIVATE_TYPED_IDEMPOTENT','PASS':True})
 command('mixed-separate');command('new-terminal');eventually(lambda:len(identities())==5);all_ids=identities();all_pids=pids()
 restored=api(['restore','--id',snapshot])['result'];assert restored['restored']and not restored['remote_connections_started']and not restored['terminal_processes_started'];assert identities()==all_ids and pids()==all_pids
 state=listing();assert all(t['id']in {t['id']for t in state['tabs']}for t in original['tabs'])
 cases.append({'id':'RESTORE_MIXED_LAYOUT_PRESERVES_ADDITIONAL_VIEWS_AND_PIDS','PASS':True})
 # Close a whole saved terminal tab; opening its snapshot reattaches the same PTY.
 single=next(t for t in original['tabs']if sum(v['tab']==t['id']for v in original['views'])==1 and any(v['tab']==t['id']and v['kind']=='terminal'for v in original['views']));index=next(i for i,t in enumerate(state['tabs'])if t['id']==single['id']);command('close-tab',tab=index);eventually(lambda:len(identities())==4)
 opened=api(['open','--id',snapshot])['result'];assert not opened['restored'];assert identities()==all_ids and pids()==all_pids
 assert api(['open','--id',snapshot])['result']['opened_tabs']==[]
 cases.append({'id':'OPEN_CLOSED_VIEW_REATTACHES_EXISTING_PROCESS','PASS':True})
 command('workspace-permission',enabled=False);assert api(['restore','--id',snapshot],False)['error']['code']=='PERMISSION_DENIED';assert identities()==all_ids
 command('workspace-permission',enabled=True)
 assert raw('workspace.restore',{'id':'../private'})['error']['code']=='INVALID_TARGET'
 assert raw('workspace.open',{'id':snapshot,'password':'NO_AUDIT_SENTINEL'})['error']['code']=='INVALID_REQUEST'
 assert 'NO_AUDIT_SENTINEL'not in(root/'gui.log').read_text()
 cases.append({'id':'LIFECYCLE_PERMISSION_AND_PAYLOAD_BOUNDARIES','PASS':True})
 # Native management works without granting IPC lifecycle permission.
 command('workspace-permission',enabled=False)
 assert command('workspace-picker',operation='show')['clicked']
 eventually(lambda:'No saved'not in command('workspace-picker',operation='inspect').get('status','Loading') and 'Loading'not in command('workspace-picker',operation='inspect').get('status','Loading'))
 command('workspace-picker',operation='save')
 eventually(lambda:'Bounded snapshot name' in command('workspace-picker',operation='inspect').get('status',''))
 command('workspace-picker',operation='name',value='Native layout')
 command('workspace-picker',operation='save')
 eventually(lambda:len(api(['saved'])['result']['saved_workspaces'])==2)
 native_id=next(v['id']for v in api(['saved'])['result']['saved_workspaces']if v['name']=='Native layout')
 assert api(['save','--name','Still denied'],False)['error']['code']=='PERMISSION_DENIED'
 command('workspace-picker',operation='close')
 cases.append({'id':'NATIVE_SAVE_VALIDATION_WITHOUT_AUTOMATION_GRANT','PASS':True})
 # Reopening the same session through Sessions creates a fresh view identity.
 state=listing();index=next(i for i,t in enumerate(state['tabs'])if t['id']==single['id'])
 old_view=next(v for v in original['views']if v['tab']==single['id'])
 before_snapshot=files[0].read_bytes()
 command('close-tab',tab=index);eventually(lambda:len(identities())==4)
 assert command('sessions-open')['clicked']
 eventually(lambda:command('sessions-open',session=old_view['session'])['clicked'])
 eventually(lambda:len(identities())==5)
 new_view=next(v for v in listing()['views']if v.get('session')==old_view['session'])
 assert new_view['id']!=old_view['id']
 aliased_ids=identities();command('workspace-permission',enabled=True)
 restored=api(['restore','--id',snapshot])['result']
 assert restored['view_identity_aliases']=={old_view['id']:new_view['id']},restored
 assert identities()==aliased_ids and pids()==all_pids
 assert files[0].read_bytes()==before_snapshot
 cases.append({'id':'REOPENED_SESSION_ALIAS_PRESERVES_PROCESS_AND_SNAPSHOT','PASS':True})
 command('workspace-permission',enabled=False)
 assert command('workspace-picker',operation='show')['clicked']
 eventually(lambda:'Loading'not in command('workspace-picker',operation='inspect').get('status','Loading'))
 assert command('workspace-picker',operation='restore',value=native_id)['clicked']
 eventually(lambda:not command('workspace-picker',operation='inspect')['open'])
 assert identities()==aliased_ids and pids()==all_pids
 assert api(['restore','--id',snapshot],False)['error']['code']=='PERMISSION_DENIED'
 cases.append({'id':'NATIVE_RESTORE_PRESERVES_REOPENED_SESSION_WITHOUT_AUTOMATION_GRANT','PASS':True})
 all_ids=aliased_ids
 command('workspace-permission',enabled=True)
 command('ipc-delay',ms=3200);result=api(['restore','--id',snapshot],False);assert result['error']['code']=='GUI_TIMEOUT';time.sleep(1.3);assert identities()==all_ids and pids()==all_pids
 cases.append({'id':'EXPIRED_REQUEST_DOES_NOT_APPLY','PASS':True})
 command('workspace-close');gui.wait(timeout=10);assert pids()==all_pids
finally:
 if gui is not None and gui.poll()is None:gui.terminate();gui.wait(timeout=10)
 if sock.exists():subprocess.run([a.binary,'--socket',str(sock),'rpc','daemon.stop'],env=env,capture_output=True,timeout=8)
 result={'cases':cases,'scope':'Actual GTK/PTY and private filesystem; synthetic local input and RDP pixels','evidence':str(root)};(root/'receipt.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result,indent=2))
