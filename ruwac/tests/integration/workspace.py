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
 path=root/'config/RUWAC/workspace.json';saved=json.loads(path.read_text());assert saved['schema']==1 and len(saved['sessions'])==3 and path.stat().st_mode&0o777==0o600
 before={s['id']:rpc('pane.list',{'session':s['id']})[0]['pid']for s in stored}
 gui=launch('restored');seq=0;assert command('inspect')['tabs']==3
 assert before=={s['id']:rpc('pane.list',{'session':s['id']})[0]['pid']for s in rpc('session.list')}
 results.append({'id':'NATIVE-WORKSPACE-REOPEN','status':'PASS','terminal_tabs':3,'persistent_pids':True,'closed_views_removed':True,'saved_mode':'0600'})
 command('profiles',search='Disposable SSH');r=command('profile-editor',operation='new',fields={'0':'Disposable SSH','3':'127.0.0.1:1','4':'fixture','13':1})
 assert r['applied'] and not r['error'],r
 assert command('profiles',search='Disposable SSH',open=True)['rows']==1
 spike.eventually(lambda:command('inspect')['tabs']==5)
 protected=command('profile-editor',operation='edit',response='delete',confirm=True)
 assert len(protected['items'])==len(r['items']),protected
 for _ in range(2):command('close-tab',tab=3);time.sleep(.3)
 removed=command('profile-editor',operation='edit',response='delete',confirm=True)
 assert len(removed['items'])==len(r['items'])-1,removed
 results.append({'id':'NATIVE-PROFILE-LEASE','status':'PASS','protected_while_open':True,'deleted_after_both_views_closed':True})
 command('workspace-close');gui.wait(timeout=10)
 path.unlink();path.symlink_to(root/'never-overwrite');gui=launch('unsafe-metadata');seq=0;command('inspect');command('workspace-close');gui.wait(timeout=10);assert path.is_symlink() and not(root/'never-overwrite').exists()
 results.append({'id':'NATIVE-WORKSPACE-UNSAFE','status':'PASS','symlink_preserved':True})
finally:
 if gui and gui.poll()is None:gui.terminate();gui.wait(timeout=5)
 if sock.exists():subprocess.run([a.binary,'--socket',str(sock),'rpc','daemon.stop','--params','{}'],capture_output=True,timeout=10)
 (root/'receipt.json').write_text(json.dumps({'results':results},indent=2));print(json.dumps({'results':results,'evidence':str(root)},indent=2))
