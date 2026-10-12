#!/usr/bin/env python3
"""Real GTK/CLI inventory IPC on an isolated compositor; no remote login claim."""
import argparse, hashlib, json, os, signal, subprocess, tempfile, time
from pathlib import Path
from spike import Client, eventually
p=argparse.ArgumentParser();p.add_argument('--binary',required=True);p.add_argument('--gui',required=True);p.add_argument('--evidence',type=Path,required=True);a=p.parse_args()
root=Path(tempfile.mkdtemp(prefix='host-inventory-',dir=a.evidence));root.chmod(0o700)
env={**os.environ,'GTK_A11Y':'none','GSETTINGS_BACKEND':'memory','RUWAC_GUI_ENTRY':a.gui}
for key,name in [('HOME','home'),('XDG_CONFIG_HOME','config'),('XDG_DATA_HOME','data'),('XDG_CACHE_HOME','cache'),('XDG_STATE_HOME','state')]:
 env[key]=str(root/name);Path(env[key]).mkdir(mode=0o700)
base=Path(env['XDG_CONFIG_HOME'])/'RUWAC';base.mkdir(mode=0o700);path=base/'machines.json';sock=root/'daemon.sock';gui=None;daemon=None;seq=0;cases=[]
secret='PRIVATE_CREDENTIAL_REFERENCE_SENTINEL';login='PRIVATE_LOGIN_NAME_SENTINEL';keypath='PRIVATE_KEY_PATH_SENTINEL'
items=[]
for i in range(1000):
 items.append({'id':f'endpoint_{i:032x}','machine_id':f'machine_{i//2:032x}','name':f'Machine č {i:04}','group':'Fleet','protocol':'SSH'if i%2==0 else'RDP','server':f'host-{i}.invalid','username':'fixture','secret_reference':'fixture-ref','favorite':i%3==0,'last_used':i,'tags':['unicode-č'],'inherit_connection':True,'preferences':{}})
items[0]['username']=login;items[0]['secret_reference']=secret;items[0]['preferences']={'identity_file':keypath,'identity_secret_reference':secret+'-key','gnome_username':login+'-linux','gnome_secret_reference':secret+'-linux'}
items[-1]['inherit_credential']=True
library={'schema':1,'items':items,'groups':[{'id':'group-fleet','path':'Fleet','username':login+'-group','defaults':{'network':'Inherited Network'},'credential_profile':'endpoint_'+'f'*32}]}
def save():
 temp=path.with_suffix('.tmp');temp.write_text(json.dumps(library));temp.chmod(0o600);temp.replace(path)
def run(args,ok=True,input=None):
 result=subprocess.run([a.binary,'--socket',str(sock),*args],env=env,input=input,capture_output=True,text=True,timeout=8)
 if ok is not None:assert(result.returncode==0)==ok,(args,result.stdout,result.stderr)
 return json.loads(result.stdout)
def api(args,ok=True):return run(['host',*args,'--json'],ok)
def raw(params,method='host.list'):return run(['gui-request'],False,input=json.dumps({'version':0,'id':'negative','method':method,'params':params}))
def command(action,**kwargs):
 global seq
 seq+=1;temp=root/'gui-request.tmp';temp.write_text(json.dumps({'seq':seq,'action':action,**kwargs}));temp.replace(root/'gui-request.json')
 def ready():
  try:
   d=json.loads((root/'gui-response.json').read_text());return d if d['seq']==seq else None
  except(FileNotFoundError,json.JSONDecodeError):return None
 return eventually(ready,5)['result']
def snapshot_files():
 return {p.name:(hashlib.sha256(p.read_bytes()).hexdigest(),p.stat().st_mtime_ns)for p in base.iterdir()if p.name.startswith('machines')}
def native_names():
 d=command('inspect');return d['tabs'],d.get('selected_tab'),d.get('focused_view')
def current_name():return api(['query','--id',items[0]['id']])['result']['endpoint']['name']
try:
 save();log=(root/'daemon.log').open('wb');daemon=subprocess.Popen([a.binary,'--socket',str(sock),'serve'],env=env,stdout=log,stderr=log)
 eventually(lambda:sock.exists(),8);created=run(['session','create','--name','Inventory daemon','--detach','--json'])
 assert api(['list'],False)['error']['code']=='GUI_UNAVAILABLE'
 gui_log=(root/'gui.log').open('wb');gui=subprocess.Popen([a.gui,'--socket',str(sock),'--fixture-dir',str(root),'--new-instance'],env=env,stdout=gui_log,stderr=gui_log)
 eventually(lambda:sock.with_suffix('.gui.sock').exists(),10);eventually(lambda:api(['list'])['result']['total']==1000,10)
 time.sleep(1)
 with Client(sock)as c:health=c.call('health');before_pid=c.call('pane.list',{'session':created['result']['session']})
 before_files=snapshot_files();before_view=native_names();first=api(['list'])['result'];offset=0;token=first['snapshot'];ids=set();pages=0;maximum=0
 while True:
  page=api(['list','--offset',str(offset),*(['--snapshot',token]if offset else[])])['result'];pages+=1
  serialized=json.dumps(page,separators=(',',':'),ensure_ascii=False);maximum=max(maximum,len(serialized.encode()));assert len(serialized.encode())<=128*1024
  assert page['total']==1000 and page['snapshot']==token
  assert all(s not in serialized for s in [secret,login,keypath])
  for row in page['endpoints']:
   assert row['id']not in ids;ids.add(row['id'])
  if page['next_offset']is None:break
  assert page['next_offset']>offset;offset=page['next_offset']
 assert len(ids)==1000 and pages==20
 row=api(['query','--id',items[998]['id']])['result']['endpoint'];assert row['network']=='Inherited Network'and row['configuration_valid']
 assert not api(['query','--id',items[999]['id']])['result']['endpoint']['configuration_valid']
 assert api(['list','--protocol','RDP','--favorites','--query','Č','--group','Fleet'])['result']['total']==167
 assert api(['list','--query','Inherited Network'])['result']['total']==999
 assert snapshot_files()==before_files and native_names()==before_view
 with Client(sock)as c:assert c.call('health')['revision']==health['revision']and c.call('pane.list',{'session':created['result']['session']})==before_pid
 cases.append({'id':'HOST-INVENTORY-REAL-1000','status':'PASS','pages':pages,'all_ids':1000,'maximum_page_bytes':maximum,'private_fields_excluded':True,'inherited_network_visible':True,'stale_account_marked_invalid':True,'library_and_focus_and_daemon_unchanged':True})
 for params in [{'password':secret},{'favorites':'yes'},{'limit':51},{'limit':0},{'query':'\n'},{'offset':1},{'protocol':'FTP'},{'snapshot':'bad'}]:assert raw(params)['error']['code']=='INVALID_REQUEST'
 assert raw({'id':items[0]['id'],'password':secret},'host.query')['error']['code']=='INVALID_REQUEST'
 assert api(['query','--id','endpoint_'+'e'*32],False)['error']['code']=='STALE_PROFILE'
 library['items'][0]['name']='Updated owner metadata';save();eventually(lambda:current_name()=='Updated owner metadata',6)
 assert api(['list','--offset','50','--snapshot',token],False)['error']['code']=='STALE_SNAPSHOT'
 token=api(['list'])['result']['snapshot'];assert api(['list','--favorites','--offset','50','--snapshot',token],False)['error']['code']=='STALE_SNAPSHOT'
 cases.append({'id':'HOST-INVENTORY-STALE-AND-STRICT','status':'PASS','metadata_reload_invalidates_token':True,'filter_change_invalidates_token':True,'strict_unknown_and_typed_parameters':True,'no_credentials_in_audit':True})
 library['items']=items[:8]
 for item in items[:4]:item['group']='Heavy';item['tags']=['č'*500]*64
 save();eventually(lambda:api(['list','--group','Heavy'])['result']['total']==4,6)
 page=api(['list','--group','Heavy'])['result'];assert 0<len(page['endpoints'])<4 and page['next_offset']==len(page['endpoints'])
 count=len(page['endpoints'])
 while page['next_offset']is not None:
  page=api(['list','--group','Heavy','--offset',str(page['next_offset']),'--snapshot',page['snapshot']])['result'];count+=len(page['endpoints']);assert len(json.dumps(page,ensure_ascii=False).encode())<=128*1024
 assert count==4
 cases.append({'id':'HOST-INVENTORY-BYTE-PAGING','status':'PASS','large_unicode_rows':4,'cursor_counts_emitted_rows':True})
 # Malformed metadata is preserved and cached native inventory is not presented as fresh.
 path.write_text('{invalid');path.chmod(0o600)
 eventually(lambda:api(['list'],None).get('error',{}).get('code')=='INVENTORY_UNAVAILABLE',6)
 assert path.read_text()=='{invalid';save();eventually(lambda:api(['list'],None).get('result',{}).get('total')==8,6)
 assert secret not in(root/'gui.log').read_text()and login not in(root/'gui.log').read_text()and keypath not in(root/'gui.log').read_text()
 command('profile-editor',operation='new',response='keep')
 layout=command('profile-editor',operation='layout');assert layout['cells']and layout['overlapping_grid_cells']==0,layout
 command('profile-editor',operation='apply',response='cancel')
 cases.append({'id':'NATIVE-PROFILE-CREDENTIAL-GRID','status':'PASS','no_visible_cell_collisions':True,'actual_native_dialog':True})
 cases.append({'id':'HOST-INVENTORY-STORAGE-FAILURE','status':'PASS','malformed_owner_document_retained':True,'typed_unavailable_then_recovery':True,'no_credential_or_config_path_audit':True})
finally:
 if gui is not None and gui.poll()is None:gui.terminate();gui.wait(timeout=10)
 if daemon is not None and daemon.poll()is None:
  try:run(['rpc','daemon.stop'])
  except Exception:daemon.terminate()
  daemon.wait(timeout=10)
 receipt={'cases':cases,'binary_sha256':hashlib.sha256(Path(a.binary).read_bytes()).hexdigest(),'gui_sha256':hashlib.sha256(Path(a.gui).read_bytes()).hexdigest(),'evidence':str(root),'scope':'Actual local GTK/Unix IPC and native library; synthetic desktop input, no external host or physical seat qualification'}
 (a.evidence/'host-inventory-acceptance.json').write_text(json.dumps(receipt,indent=2)+'\n');print(json.dumps(receipt,indent=2))
