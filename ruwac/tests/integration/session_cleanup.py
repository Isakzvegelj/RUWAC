#!/usr/bin/env python3
"""Real PTYs: final exit, safe dead-window cleanup, and named-session restart."""
import argparse,json,os,subprocess,tempfile,time
from pathlib import Path
from spike import Client,Tui,eventually
p=argparse.ArgumentParser();p.add_argument('--binary',type=Path,required=True);p.add_argument('--daemon',type=Path);p.add_argument('--evidence',type=Path,required=True);a=p.parse_args()
r=Path(tempfile.mkdtemp(prefix='cleanup-',dir=a.evidence)).resolve();r.chmod(0o700);sock=r/'control.sock';env={**os.environ,'TERM':'xterm-256color','RUWAC_STATE_FILE':str(r/'state.json')};views=[];checks=[]
for key in ['HOME','XDG_CONFIG_HOME','XDG_DATA_HOME','XDG_CACHE_HOME']:
 folder=r/key.lower();folder.mkdir(mode=0o700);env[key]=str(folder)
shell=r/'selected-shell';shell.write_text('#!/bin/sh\nprintf NAMED_FRESH_READY\nexec /bin/cat\n');shell.chmod(0o700);env['SHELL']=str(shell)
log=(r/'daemon.log').open('w');daemon=subprocess.Popen([str(a.daemon or a.binary),'--socket',str(sock),'serve'],env=env,stdout=log,stderr=log)
def rpc(m,p=None,error=None):
 with Client(sock)as c:return c.call(m,p,error=error)
def cli(*args):return subprocess.run([str(a.binary),'--socket',str(sock),*args],env=env,capture_output=True,text=True,timeout=12)
def dead(name):
 s=rpc('session.create',{'name':name,'argv':['/bin/sh','-c','printf GENERATED_OLD_HISTORY']});eventually(lambda:rpc('pane.list',{'session':s['session']})[0]['ended']);return s
try:
 eventually(sock.exists)
 d1,d2=dead('dead-one'),dead('dead-two');live=rpc('session.create',{'name':'live','argv':['/bin/cat']});original_pid=live['pid'];w=rpc('window.create',{'session':live['session'],'name':'dead-window','argv':['/bin/sh','-c','true']});eventually(lambda:next(p for p in rpc('pane.list',{'session':live['session']})if p['id']==w['pane'])['ended'])
 assert 'dead' in cli('-ls').stdout
 q=cli('-wipe','dead-one','--json');assert q.returncode==0,(q.stdout,q.stderr);assert json.loads(q.stdout)['removed_sessions']==1
 assert any(s['id']==d2['session']for s in rpc('session.list'))
 q=cli('-wipe','--json');assert q.returncode==0,(q.stdout,q.stderr);report=json.loads(q.stdout);assert report['removed_sessions']==1 and report['removed_windows']==2
 assert [p['id']for p in rpc('pane.list',{'session':live['session']})]==[live['pane']]
 assert rpc('pane.list',{'session':live['session']})[0]['pid']==original_pid;rpc('pane.sendText',{'pane':live['pane'],'text':'LIVE_INPUT_RETAINED\n'});eventually(lambda:'LIVE_INPUT_RETAINED'in rpc('pane.capture',{'pane':live['pane']})['text'])
 checks.append('named-and-global-wipe-remove-only-dead-windows-with-live-PID-and-input-preserved')
 schema=rpc('version')
 if a.daemon is None:
  rpc('pane.close',{'pane':live['pane'],'if_ended':True},error='PROCESS_RUNNING');assert rpc('pane.list',{'session':live['session']})[0]['pid']==original_pid
 checks.append('live-pane-close-guard-or-older-engine-ended-ID-compatibility')
 s=rpc('session.create',{'name':'fresh-exit','argv':['/bin/sh','-c','printf FRESH_READY; exec /bin/sh']})
 for n in range(2):
  v=Tui(a.binary,sock,s['session'],r/('final-'+str(n)+'.pty'),env=env,command=['-r',s['session']]);views.append(v);eventually(lambda:b'FRESH_READY'in v.data)
 views[-1].send(b'exit\r');eventually(lambda:all(v.p.poll()is not None for v in views),6)
 while views:
  v=views.pop(0);assert v.finish()==0,{'client_exit':v.p.returncode,'transcript':str(v.path)}
 assert all(row['id']!=s['session']for row in rpc('session.list'));checks.append('fresh-final-shell-exit-removes-session-and-both-clients-return-cleanly')
 old=dead('reopen-dead');v=Tui(a.binary,sock,old['session'],r/'restart.pty',env=env,command=['-R','reopen-dead']);views.append(v);eventually(lambda:b'NAMED_FRESH_READY'in v.data)
 new=next(s for s in rpc('session.list')if s['name']=='reopen-dead');assert new['id']!=old['session'];assert rpc('pane.list',{'session':new['id']})[0]['ended']is False;assert v.finish()==0;views.clear();checks.append('attach-or-create-replaces-an-ended-session-with-a-fresh-live-shell')
 before=rpc('session.list');q=cli('-wipe','missing-name');assert q.returncode!=0 and rpc('session.list')==before;assert cli('-wipe','--json').returncode==0;checks.append('missing-name-refused-and-repeated-wipe-preserves-all-live-sessions')
 print('Session cleanup PASS',len(checks),'groups');(r/'acceptance.json').write_text(json.dumps({'PASS':True,'checks':checks},indent=2)+'\n')
finally:
 for v in views:v.finish('term')
 if daemon.poll()is None:daemon.terminate();daemon.wait(timeout=8)
 log.close()
