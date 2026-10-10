#!/usr/bin/env python3
"""Bounded SIGKILL/raw-mode regression using only a new daemon and owned PTY."""
import argparse,hashlib,importlib.util,json,os,shlex,signal,subprocess,sys,tempfile,time
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('--binary',required=True);p.add_argument('--evidence',required=True);a=p.parse_args()
spec=importlib.util.spec_from_file_location('spike',Path(__file__).with_name('spike.py'));spike=importlib.util.module_from_spec(spec);spec.loader.exec_module(spike)
root=Path(tempfile.mkdtemp(prefix='abrupt-child-',dir=a.evidence));root.chmod(0o700);sock=root/'control.sock';fixture=Path(__file__).parents[1]/'fixtures/termios_child.py';log=(root/'daemon.log').open('wb');daemon=subprocess.Popen([a.binary,'--socket',str(sock),'serve'],stdout=log,stderr=log)
try:
 spike.eventually(lambda:sock.exists())
 with spike.Client(sock) as c:
  s=c.call('session.create',{'name':'owned raw recovery','argv':['env','PS1=RUWAC-PROMPT>','bash','--noprofile','--norc','-i']});c.call('session.attach',{'session':s['session']});
  c.call('pane.sendText',{'pane':s['pane'],'text':shlex.join([sys.executable,str(fixture),'raw',str(root/'child.json')])+'\r'});spike.eventually(lambda:(root/'child.json').exists());child=json.loads((root/'child.json').read_text());assert child['sid']==s['pid'];assert os.getsid(child['pid'])==s['pid'];spike.eventually(lambda:next(p['grid'] for p in c.call('client.render')['panes'] if p['id']==s['pane'])['mouse']);os.kill(child['pid'],signal.SIGKILL)
  def restored_grid():
   grid=next(p['grid'] for p in c.call('client.render')['panes'] if p['id']==s['pane']);return grid if not grid['mouse'] and not grid['mouse_sgr'] else False
  spike.eventually(restored_grid,5)
  # Disabled mouse input must not leak ESC[< bytes into the shell.
  c.call('client.input',{'pane':s['pane'],'kind':'mouse','text':'','mouse_event':{'button':0,'x':2,'y':1,'release':False}})
  c.call('pane.sendText',{'pane':s['pane'],'text':shlex.join([sys.executable,str(fixture),'probe',str(root/'probe.json')])+'\r'});spike.eventually(lambda:(root/'probe.json').exists(),5);state=json.loads((root/'probe.json').read_text());assert state['echo'] and state['canonical'],state
  data='Živjo raw-child recovery\n'.encode();c.call('pane.sendText',{'pane':s['pane'],'text':data.decode()});state=spike.eventually(lambda:r if 'sha256' in (r:=json.loads((root/'probe.json').read_text())) else False);assert state['sha256']==hashlib.sha256(data).hexdigest(),state
  report={'status':'PASS','cases':['Owned raw child SIGKILL','Foreground mouse/SGR reset; preserve Bash readline bracketed paste','Disabled mouse no escape leakage','Bash restores ECHO/ICANON','Exact Unicode probe input'],'limit':'Bash/job-control fixture only; general termios recovery and physical input remain unqualified','probe':state}
 (Path(a.evidence)/'abrupt-child.json').write_text(json.dumps(report,indent=2));print(json.dumps(report))
finally:
 daemon.terminate()
 try:daemon.wait(timeout=5)
 except subprocess.TimeoutExpired:daemon.kill();daemon.wait()
 log.close()
