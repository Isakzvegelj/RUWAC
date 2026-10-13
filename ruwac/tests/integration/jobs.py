#!/usr/bin/env python3
"""Owned real-PTY process metadata, privacy, exit and recovery acceptance."""
import argparse, hashlib, json, os, signal, subprocess, sys, tempfile, time
from pathlib import Path
from spike import Client, eventually

p=argparse.ArgumentParser();p.add_argument('--binary',required=True);p.add_argument('--evidence',type=Path,required=True);a=p.parse_args()
root=Path(tempfile.mkdtemp(prefix='jobs-',dir=a.evidence));root.chmod(0o700);sock=root/'control.sock';binary=Path(a.binary).resolve();results=[];daemon=None
env={**os.environ,'RUWAC_STATE_FILE':str(root/'checkpoint.json'),'HOME':str(root/'home')};Path(env['HOME']).mkdir()
def start():
 global daemon
 daemon=subprocess.Popen([str(binary),'--socket',str(sock),'serve'],env=env,stdout=(root/'daemon.log').open('ab'),stderr=subprocess.STDOUT)
 eventually(lambda:sock.exists() and rpc('health')['ok'])
def rpc(method,params=None,**kw):
 with Client(sock) as c:return c.call(method,params,**kw)
def job(pane):return rpc('job.query',{'pane':pane})
try:
 start();cwd=root/'foreground';cwd.mkdir();fixture=root/'child.py'
 fixture.write_text('''import os,signal,sys,time,tty
signal.signal(signal.SIGTTOU,signal.SIG_IGN)
pid=os.fork()
if pid==0:
 os.setpgid(0,0);os.chdir(sys.argv[1]);tty.setraw(0)
 while os.tcgetpgrp(0)!=os.getpid():time.sleep(.01)
 print("JOB_READY",flush=True)
 os.read(0,1);os._exit(0)
os.setpgid(pid,pid);os.tcsetpgrp(0,pid)
os.waitpid(pid,0);os.tcsetpgrp(0,os.getpgrp())
print("JOB_DONE",flush=True);sys.exit(23)
''')
 s=rpc('session.create',{'name':'Job acceptance','argv':[sys.executable,str(fixture),str(cwd),'PRIVATE-ARG-NOT-METADATA']})
 rpc('pane.wait',{'pane':s['pane'],'contains':'JOB_READY','timeout_ms':5000})
 j=eventually(lambda:(v if (v:=job(s['pane']))['foreground'] and v['foreground']['pid']!=v['pid'] else None))
 assert j['foreground']['cwd']==str(cwd) and j['pid']==s['pid'] and j['phase']=='running',j
 assert j['host'] and j['command'] and j['age_ms']>=0 and j['last_output_ms'],j
 assert 'PRIVATE-ARG-NOT-METADATA' not in json.dumps(j) and j['process_metadata_limit']['argv'] is False
 time.sleep(1);assert job(s['pane'])['phase']=='running' and job(s['pane'])['agent_state']=='unknown'
 results.append({'id':'JOBS-LIVE-FOREGROUND-PRIVACY-QUIET','status':'PASS','real_foreground_process_group':True})
 before=rpc('health')['revision'];page=rpc('job.list',{'query':'Job acceptance','limit':1});assert page['total']==1 and page['jobs'][0]['id']==s['pane']
 job(s['pane']);assert rpc('health')['revision']==before
 probe=rpc('agent.probe',{'pane':s['pane']});assert not probe['process_detected'] and probe['foreground_pid']==j['foreground']['pid'] and set(probe)=={'pane','adapter','foreground_pid','process_detected','selected'}
 assert rpc('health')['revision']==before
 rpc('agent.probe',{'pane':s['pane'],'argv':True},error='INVALID_PARAMS');rpc('agent.probe',{'pane':'window_'+'a'*32},error='INVALID_ID');rpc('agent.probe',{'pane':'pane_'+'a'*32},error='STALE_ID');rpc('agent.probe',{'pane':s['pane']},key='stale-probe',error='INVALID_PARAMS')
 for params in [{'limit':0},{'limit':101},{'offset':129},{'query':'x'*257},{'argv':True}]:rpc('job.list',params,error='INVALID_PARAMS')
 rpc('job.query',{'pane':'window_'+'a'*32},error='INVALID_ID');rpc('job.query',{'pane':'pane_'+'a'*32},error='STALE_ID')
 rpc('job.query',{'pane':s['pane']},key='fresh-metadata',error='INVALID_PARAMS')
 cli=subprocess.run([str(binary),'--socket',str(sock),'job','query','--id',s['pane'],'--json'],capture_output=True,timeout=5)
 assert cli.returncode==0 and json.loads(cli.stdout)['result']['id']==s['pane'],cli.stderr
 results.append({'id':'JOBS-READONLY-BOUNDS-CLI','status':'PASS'})
 rpc('pane.sendText',{'pane':s['pane'],'text':'x'});j=eventually(lambda:(v if (v:=job(s['pane']))['phase']=='exited' else None));assert j['exit_code']==23 and j['foreground'] is None,j
 results.append({'id':'JOBS-REAL-EXIT-STATUS','status':'PASS','exit_code':23})
 rpc('daemon.checkpoint');daemon.kill();daemon.wait(timeout=5);start()
 j=job(s['pane']);assert j['phase']=='restored_history' and j['pid'] is None and j['command'] is None and j['cwd'] is None and j['foreground'] is None,j
 assert b'PRIVATE-ARG-NOT-METADATA' not in (root/'checkpoint.json').read_bytes()
 results.append({'id':'JOBS-CRASH-RESTORED-HISTORY-NO-PROCESS','status':'PASS'})
finally:
 if daemon is not None and daemon.poll() is None:
  try:rpc('daemon.stop');daemon.wait(timeout=8)
  except Exception:daemon.kill();daemon.wait()
 receipt={'results':results,'binary_sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),'scope':'Real owned daemon and PTY processes, no owner sessions or graphical assumptions'}
 (root/'receipt.json').write_text(json.dumps(receipt,indent=2)+'\n');print(json.dumps(receipt,indent=2),flush=True)
assert len(results)==4
