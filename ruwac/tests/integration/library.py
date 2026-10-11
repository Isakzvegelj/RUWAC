#!/usr/bin/env python3
"""Actual GTK paging and reviewed explicit-identity Remmina merge; private metadata only."""
import argparse,hashlib,json,os,signal,subprocess,tempfile,time
from pathlib import Path
from spike import Client,eventually
p=argparse.ArgumentParser();p.add_argument('--binary',required=True);p.add_argument('--gui',required=True);p.add_argument('--evidence',type=Path,required=True);a=p.parse_args()
root=Path(tempfile.mkdtemp(prefix='library-',dir=a.evidence));root.chmod(0o700);env={**os.environ,'GTK_A11Y':'none','GSETTINGS_BACKEND':'memory'}
for key,name in [('HOME','home'),('XDG_CONFIG_HOME','config'),('XDG_DATA_HOME','data'),('XDG_CACHE_HOME','cache')]:
 env[key]=str(root/name);Path(env[key]).mkdir(mode=0o700)
folder=Path(env['XDG_CONFIG_HOME'])/'RUWAC';folder.mkdir(mode=0o700);store=folder/'machines.json';sock=root/'control.sock';gui=None;daemon=None;seq=0;results=[]
items=[{'id':f'endpoint_{i:032x}','machine_id':f'machine_{i:032x}','name':f'Load{i:03}','group':'Library/Nested','protocol':'RDP','server':f'127.0.0.1:{3389+i}','username':'fixture','secret_reference':f'private-reference-{i}','favorite':False,'last_used':123,'tags':['keep'],'inherit':True,'preferences':{'label':'local label','network':'','gateway':'','trust':'local trust','identity_file':'local key','known_hosts':'','clipboard':2,'display':'fit','zoom':1.25,'fullscreen':False,'reconnect':True}}for i in range(1000)]
for i in [0,1]:items[i]['name']='Same leaf'
store.write_text(json.dumps({'schema':1,'items':items,'groups':[]},separators=(',',':')));store.chmod(0o600)
source='[remmina]\nname=Same leaf\ngroup=Imported/Reviewed\nprotocol=SSH\nserver=localhost:2222\nusername=other\nlabels=new,imported\npassword=NEVER-IMPORT\nssh_privatekey=NEVER-IMPORT-KEY\n'
file=root/'import.remmina';file.write_text(source)
def command(action,**kw):
 global seq
 seq+=1;tmp=root/'request.tmp';tmp.write_text(json.dumps({'seq':seq,'action':action,**kw}));tmp.replace(root/'gui-request.json')
 def read():
  try:d=json.loads((root/'gui-response.json').read_text());return d['result']if d['seq']==seq else None
  except (OSError,ValueError):return None
 return eventually(read,10)
def control(**kw):return command('profiles-control',**kw)
def metadata():return json.loads(store.read_text())['items']
def snapshot():return {p['id']:p for p in metadata()}
def preview(target=None):
 control(file=file.name)
 if target:control(target=target)
def reviewed(target):preview(target);r=control(operation='review');assert r['review_text']and target in r['review_text'];return r
try:
 daemon=subprocess.Popen([a.binary,'--socket',str(sock),'serve'],env=env,stdout=(root/'daemon.log').open('wb'),stderr=subprocess.STDOUT);eventually(lambda:sock.exists())
 with Client(sock)as c:session=c.call('session.create',{'name':'Library acceptance','argv':['/bin/sh','-c','sleep 300']})
 gui=subprocess.Popen([a.gui,'--socket',str(sock),'--session',session['session'],'--fixture-dir',str(root)],env=env,stdout=(root/'gui.log').open('wb'),stderr=subprocess.STDOUT)
 eventually(lambda:command('inspect'),15)
 state=eventually(lambda:(r if (r:=control())['list_height']>0 else None),10)
 assert not state['import_expanded']and not state['tools_expanded'],state
 assert state['list_height']>=state['sidebar_height']*0.50,state
 control(import_expanded=True,tools_expanded=True)
 assert control()['import_expanded']and control()['tools_expanded']
 control(import_expanded=False,tools_expanded=False)
 results.append({'id':'LIBRARY-COMPACT-NATIVE-SIDEBAR','status':'PASS','less_used_panels_collapsed_by_default':True,'list_gets_majority_of_sidebar':True,'native_expanders_open_and_close':True,'allocated_list_height':state['list_height'],'allocated_sidebar_height':state['sidebar_height']})
 start=time.perf_counter();seen=[]
 for page in range(20):
  r=control();assert r['visible']['rows']==50,r;assert r['page']==f'{page*50+1}–{page*50+50} of 1000',r
  seen.extend(r['visible']['labels']);assert r['previous']==(page>0)and r['next']==(page<19)
  control(operation='next')
 assert len(seen)==len(set(seen))==1000
 command('profiles',search='Load037');r=control();assert r['page']=='1–1 of 1'and r['visible']['rows']==1 and not r['next']
 command('profiles',search='');assert control()['page']=='1–50 of 1000'
 results.append({'id':'LIBRARY-NATIVE-PAGING-1000','status':'PASS','pages':20,'visible_row_cap':50,'all_endpoints_reachable':True,'search_resets_page':True,'elapsed_ms':(time.perf_counter()-start)*1000})
 before=store.read_bytes();preview();r=control(operation='review');assert r['review_text']is None and 'explicitly'in r['status'];assert store.read_bytes()==before
 target=items[37]['id'];r=reviewed(target);assert 'Load037 → Same leaf'in r['review_text']and 'localhost:2222'in r['review_text'];control(operation='cancel');assert store.read_bytes()==before
 results.append({'id':'LIBRARY-MERGE-EXPLICIT-REVIEW-CANCEL','status':'PASS','same_name_never_selects_identity':True,'native_dialog_diff':True,'cancel_no_write':True})
 baseline=snapshot();reviewed(target);r=control(operation='apply');after=snapshot();assert len(after)==1000 and set(after)==set(baseline)
 for key in ['machine_id','secret_reference','preferences','favorite','last_used','inherit']:assert after[target][key]==baseline[target][key],key
 assert after[target]['protocol']=='SSH'and after[target]['server']=='localhost:2222'and after[target]['username']=='other'
 assert all(after[k]==v for k,v in baseline.items()if k!=target)
 assert file.read_text()==source and 'NEVER-IMPORT'not in store.read_text()
 stat_mode=(store.stat().st_mode&0o777)==0o600;assert stat_mode
 results.append({'id':'LIBRARY-MERGE-PERSISTED-IDENTITY-POLICY','status':'PASS','profile_cap_does_not_block_merge':True,'one_explicit_endpoint_changed':True,'credential_references_and_local_policy_preserved':True,'source_untouched':True,'private_store_mode':stat_mode})
 control(operation='undo');assert snapshot()==baseline
 reviewed(target);control(operation='apply');assert snapshot()[target]['server']=='localhost:2222'
 results.append({'id':'LIBRARY-MERGE-UNDO','status':'PASS','native_undo_restores_previous_library':True})
 target2=items[38]['id'];reviewed(target2);command('profiles',search='Load038',favorite=True);changed=store.read_bytes();r=control(operation='apply');assert 'changed or became active'in r['status']and store.read_bytes()==changed
 results.append({'id':'LIBRARY-MERGE-STALE-REVIEW','status':'PASS','changed_profile_requires_fresh_review':True})
 preview(target2);command('profiles',search='Load038',open=True);r=control(operation='review');assert r['review_text']is None and 'active connection'in r['status'];active=store.read_bytes();assert active
 results.append({'id':'LIBRARY-MERGE-ACTIVE-ENDPOINT','status':'PASS','open_connection_profile_protected':True,'no_external_login_claim':True})
 command('workspace-close');gui.wait(timeout=10)
 gui=subprocess.Popen([a.gui,'--socket',str(sock),'--session',session['session'],'--fixture-dir',str(root)],env=env,stdout=(root/'restarted.log').open('wb'),stderr=subprocess.STDOUT)
 eventually(lambda:command('inspect'),15);assert snapshot()[target]['server']=='localhost:2222'
 results.append({'id':'LIBRARY-MERGE-RESTART','status':'PASS','persisted_imported_endpoint':True})
finally:
 if gui is not None and gui.poll()is None:gui.terminate();gui.wait(timeout=10)
 if daemon is not None and daemon.poll()is None:
  try:
   with Client(sock)as c:c.call('daemon.stop')
   daemon.wait(timeout=5)
  except Exception:daemon.kill();daemon.wait()
 receipt={'results':results,'hashes':{n:hashlib.sha256(Path(path).read_bytes()).hexdigest()for n,path in [('ruwac',a.binary),('ruwac-desktop',a.gui)]},'evidence':str(root),'scope':'Actual native GTK widget/dialog callbacks on an owned compositor; synthetic actions; no owner credentials or physical input'}
 (root/'receipt.json').write_text(json.dumps(receipt,indent=2)+'\n');print(json.dumps(receipt,indent=2))
assert len(results)==8
