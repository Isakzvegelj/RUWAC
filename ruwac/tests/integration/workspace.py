#!/usr/bin/env python3
"""Native startup/reopen/close and metadata protection on an owned GTK compositor."""
import argparse,importlib.util,json,os,subprocess,sys,tempfile,time
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('--binary',required=True);p.add_argument('--evidence',required=True);a=p.parse_args()
spec=importlib.util.spec_from_file_location('spike',Path(__file__).with_name('spike.py'));spike=importlib.util.module_from_spec(spec);spec.loader.exec_module(spike)
root=Path(tempfile.mkdtemp(prefix='workspace-',dir=a.evidence));root.chmod(0o700);env={**os.environ,'HOME':str(root/'home'),'XDG_CONFIG_HOME':str(root/'config'),'XDG_DATA_HOME':str(root/'data'),'GTK_A11Y':'none'}
for key in ['HOME','XDG_CONFIG_HOME','XDG_DATA_HOME']:Path(env[key]).mkdir(mode=0o700)
sock=root/'daemon.sock';seq=0;gui=None;results=[]
def launch(name):return subprocess.Popen([a.binary,'--socket',str(sock),'gui','--fixture-dir',str(root)],env=env,stdout=(root/(name+'.log')).open('wb'),stderr=subprocess.STDOUT)
def command(action,**kw):
 global seq
 seq+=1;temp=root/'gui-request.tmp';temp.write_text(json.dumps({'seq':seq,'action':action,**kw}));temp.replace(root/'gui-request.json')
 def result():
  try:r=json.loads((root/'gui-response.json').read_text());return r['result']if r['seq']==seq else None
  except (FileNotFoundError,ValueError):return None
 return spike.eventually(result,10)
def rpc(method,params=None):
 with spike.Client(sock)as c:return c.call(method,params)
try:
 gui=launch('first');spike.eventually(lambda:sock.exists());assert command('inspect')['tabs']==1
 initial=rpc('session.list');assert len(initial)==1
 for count in [2,3]:
  assert command('new-terminal')['clicked'];spike.eventually(lambda:command('inspect')['tabs']==count)
 stored=rpc('session.list');assert len(stored)==3
 command('close-tab',tab=1);spike.eventually(lambda:command('inspect')['tabs']==2);assert len(rpc('session.list'))==3
 assert len(command('inspect')['panes'])==2
 command('sessions-open');time.sleep(.2)
 # Reopen the closed middle tab by stable ID, using child creation order.
 ordered=sorted(stored,key=lambda s:rpc('pane.list',{'session':s['id']})[0]['pid']);closed=ordered[1]
 assert command('sessions-open',session=closed['id'])['clicked'];spike.eventually(lambda:command('inspect')['tabs']==3)
 command('workspace-close');gui.wait(timeout=10)
 path=root/'config/RUWAC/workspace.json';saved=json.loads(path.read_text());assert saved['schema']==3 and len(saved['sessions'])==3 and path.stat().st_mode&0o777==0o600
 before={s['id']:rpc('pane.list',{'session':s['id']})[0]['pid']for s in stored}
 gui=launch('restored');seq=0;assert command('inspect')['tabs']==3
 assert before=={s['id']:rpc('pane.list',{'session':s['id']})[0]['pid']for s in rpc('session.list')}
 results.append({'id':'NATIVE-WORKSPACE-REOPEN','status':'PASS','terminal_tabs':3,'persistent_pids':True,'closed_views_removed':True,'saved_mode':'0600'})
 command('workspace-close');gui.wait(timeout=10)
 legacy={'schema':1,'socket':str(sock),'sessions':list(before),'selected':next(iter(before))}
 path.write_text(json.dumps(legacy))
 gui=launch('legacy-migration');seq=0;assert command('inspect')['tabs']==3
 spike.eventually(lambda:json.loads(path.read_text())['schema']==3,5)
 backup=path.with_name('workspace-v1.json');assert json.loads(backup.read_text())==legacy and backup.stat().st_mode&0o777==0o600
 assert before=={s['id']:rpc('pane.list',{'session':s['id']})[0]['pid']for s in rpc('session.list')}
 results.append({'id':'NATIVE-WORKSPACE-MIGRATION','status':'PASS','schema_1_to_3':True,'original_backup_mode':'0600','same_terminal_pids':True})
 command('profiles',search='Disposable SSH');r=command('profile-editor',operation='new',fields={'0':'Disposable SSH','3':'127.0.0.1:1','4':'fixture','13':1})
 assert r['applied'] and not r['error'],r
 assert command('profiles',search='Disposable SSH',open=True)['rows']==1
 spike.eventually(lambda:command('inspect')['tabs']==5)
 command('tab-select',tab=4)
 mixed=command('mixed-join',tab=3,vertical=False);assert len(mixed['tabs'])==4 and mixed['tabs'][3]['layout']['split']=='horizontal',mixed
 time.sleep(.3)
 protected=command('profile-editor',operation='edit',response='delete',confirm=True)
 assert len(protected['items'])==len(r['items']),protected
 assert command('views-manage')['clicked']
 moved_file=command('views-action',operation='move',index=1);assert moved_file['clicked'] and len(moved_file['layout']['tabs'])==5,moved_file
 time.sleep(.3)
 protected=command('profile-editor',operation='edit',response='delete',confirm=True);assert len(protected['items'])==len(r['items']),protected
 command('tab-select',tab=4);assert command('views-manage')['clicked']
 assert command('views-action',operation='close',index=0)['clicked'];time.sleep(.3)
 protected=command('profile-editor',operation='edit',response='delete',confirm=True);assert len(protected['items'])==len(r['items']),protected
 command('tab-select',tab=3);assert command('views-manage')['clicked']
 assert command('views-action',operation='close',index=0)['clicked'];time.sleep(.3)
 removed=command('profile-editor',operation='edit',response='delete',confirm=True)
 assert len(removed['items'])==len(r['items'])-1,removed
 results.append({'id':'NATIVE-PROFILE-LEASE','status':'PASS','protected_while_open':True,'deleted_after_both_views_closed':True,'mixed_reparent_kept_leases':True,'native_files_move_kept_lease':True,'native_files_close_released_only_its_lease':True,'native_terminal_close_released_last_lease':True})
 # Save/restore all view kinds and a nested mixed tree. The RDP view here is deliberately
 # disconnected/synthetic; real authenticated reparenting is qualified by rdp_clipboard.py.
 command('rdp-open')
 command('profiles',search='Saved files')
 profile=command('profile-editor',operation='new',fields={'0':'Saved files','3':'127.0.0.1:1','4':'fixture','13':1})
 assert profile['applied'] and not profile['error'],profile
 assert command('profiles',search='Saved files',open=True)['rows']==1
 spike.eventually(lambda:command('inspect')['tabs']==6)
 command('files-directory',path='/generated/directory with spaces')
 command('tab-select',tab=0)
 command('mixed-join',tab=3,vertical=False)
 mixed=command('mixed-join',tab=3,vertical=True)
 assert len(mixed['tabs'])==4 and mixed['tabs'][0]['layout']['split']=='vertical',mixed
 live_before={sid:rpc('pane.list',{'session':sid})[0]['pid']for sid in before}
 spike.eventually(lambda:len(json.loads(path.read_text()).get('tabs',[]))==4,5)
 gui.kill();gui.wait(timeout=10)
 saved=json.loads(path.read_text());assert saved['schema']==3 and len(saved['tabs'])==4 and saved['selected_tab']==0,saved
 first=saved['tabs'][0]['layout'];assert first['kind']=='split' and first['vertical'] and first['first']['kind']=='split' and first['second']['kind']=='files',first
 assert first['second']['directory']=='/generated/directory with spaces'
 assert 'password' not in path.read_text().lower()
 gui=launch('mixed-restored');seq=0;assert command('inspect')['tabs']==4
 restored=command('mixed-inspect');assert restored['tabs'][0]['layout']['split']=='vertical' and restored['tabs'][0]['layout']['first']['split']=='horizontal',restored
 assert command('files-directory')['directory']=='/generated/directory with spaces'
 assert command('rdp-inspect',rdp=0)['transport_id']is None
 assert live_before=={sid:rpc('pane.list',{'session':sid})[0]['pid']for sid in before}
 protected=command('profile-editor',operation='edit',response='delete',confirm=True)
 assert len(protected['items'])==len(profile['items']),protected
 results.append({'id':'NATIVE-MIXED-RESTORE','status':'PASS','saved_schema':3,'mixed_tab_kinds':['terminal','rdp','files'],'nested_orientations':['vertical','horizontal'],'same_terminal_pids':True,'directory_and_profile_restored':True,'credentials_not_serialized':True,'restored_rdp_disconnected':True,'restored_profile_lease':True,'autosave_survived_gui_sigkill':True})
 command('mixed-separate');command('close-tab',tab=5);time.sleep(.2);command('close-tab',tab=3);time.sleep(.2)
 removed=command('profile-editor',operation='edit',response='delete',confirm=True);assert len(removed['items'])==len(profile['items'])-1,removed
 command('workspace-close');gui.wait(timeout=10)
 original=path.read_bytes();other=sock.with_name('other.sock')
 gui=subprocess.Popen([a.binary,'--socket',str(other),'gui','--fixture-dir',str(root)],env=env,stdout=(root/'other-socket.log').open('wb'),stderr=subprocess.STDOUT);seq=0
 assert command('inspect')['tabs']==1
 spike.eventually(lambda:len(list(path.parent.glob('workspace-*.json')))==2,5) # migration backup plus one socket sidecar
 assert path.read_bytes()==original
 command('workspace-close');gui.wait(timeout=10)
 subprocess.run([a.binary,'--socket',str(other),'rpc','daemon.stop','--params','{}'],capture_output=True,check=True,timeout=10)
 gui=launch('original-socket-return');seq=0
 assert command('inspect')['tabs']==4
 assert before=={sid:rpc('pane.list',{'session':sid})[0]['pid']for sid in before}
 results.append({'id':'NATIVE-WORKSPACE-SOCKETS','status':'PASS','second_daemon_sidecar':True,'original_metadata_preserved':True,'original_live_pids_preserved':True})
 # Exercise native per-view buttons on a nested tree and preserve every daemon process.
 command('tab-select',tab=0)
 command('mixed-join',tab=1,vertical=False)
 tree=command('mixed-join',tab=1,vertical=True);assert len(tree['tabs'])==2,tree
 assert command('views-manage')['clicked']
 closed=command('views-action',operation='close',index=1);assert closed['clicked'] and closed['terminal_views']==2,closed
 tree=closed['layout'];assert len(tree['tabs'])==2 and tree['tabs'][0]['layout']['split']=='vertical' and 'view' in tree['tabs'][0]['layout']['first'],tree
 assert before=={sid:rpc('pane.list',{'session':sid})[0]['pid']for sid in before}
 command('sessions-open');spike.eventually(lambda:command('sessions-open',session=list(before)[1])['clicked'])
 spike.eventually(lambda:command('inspect')['tabs']==3)
 command('tab-select',tab=0);assert command('views-manage')['clicked']
 moved=command('views-action',operation='move',index=1);assert moved['clicked'] and len(moved['layout']['tabs'])==4 and moved['terminal_views']==3,moved
 assert 'view' in moved['layout']['tabs'][0]['layout'],moved
 assert before=={sid:rpc('pane.list',{'session':sid})[0]['pid']for sid in before}
 # Closing a now-single terminal view removes only its presentation, and reopening is possible.
 assert command('views-manage')['clicked']
 sole=command('views-action',operation='close',index=0);assert sole['clicked'] and sole['terminal_views']==2 and len(sole['layout']['tabs'])==3,sole
 command('sessions-open');spike.eventually(lambda:command('sessions-open',session=list(before)[2])['clicked'])
 spike.eventually(lambda:command('inspect')['tabs']==4)
 results.append({'id':'NATIVE-MIXED-VIEW-MANAGEMENT','status':'PASS','native_dialog_buttons':True,'nested_leaf_close_collapsed_split':True,'neighbor_pids_preserved':True,'closed_terminal_reopened_same_pid':True,'one_view_moved_to_own_tab':True,'sole_view_close_removed_tab':True})
 # Close every presentation, including the last one, then use the actual New terminal button.
 while command('inspect')['tabs']:
  command('tab-select',tab=0);assert command('views-manage')['clicked']
  assert command('views-action',operation='close',index=0)['clicked']
 assert command('inspect')['terminal_views']==0
 spike.eventually(lambda:json.loads(path.read_text())['tabs']==[],5)
 assert before=={sid:rpc('pane.list',{'session':sid})[0]['pid']for sid in before}
 assert command('new-terminal')['clicked'];spike.eventually(lambda:command('inspect')['tabs']==1)
 results.append({'id':'NATIVE-EMPTY-WORKSPACE','status':'PASS','last_view_close':True,'zero_view_checkpoint':True,'existing_daemon_pids_preserved':True,'native_new_terminal_from_empty_workspace':True})
 command('workspace-close');gui.wait(timeout=10)
 path.unlink();path.symlink_to(root/'never-overwrite');gui=launch('unsafe-metadata');seq=0;command('inspect');command('workspace-close');gui.wait(timeout=10);assert path.is_symlink() and not(root/'never-overwrite').exists()
 results.append({'id':'NATIVE-WORKSPACE-UNSAFE','status':'PASS','symlink_preserved':True})
finally:
 if gui and gui.poll()is None:gui.terminate();gui.wait(timeout=5)
 if sock.exists():subprocess.run([a.binary,'--socket',str(sock),'rpc','daemon.stop','--params','{}'],capture_output=True,timeout=10)
 (root/'receipt.json').write_text(json.dumps({'results':results},indent=2));print(json.dumps({'results':results,'evidence':str(root)},indent=2))
