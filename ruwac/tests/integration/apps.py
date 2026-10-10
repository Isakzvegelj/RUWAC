#!/usr/bin/env python3
"""Owned Vim/less/htop PTYs with isolated configuration and TUI restoration."""
import argparse,importlib.util,json,os,subprocess,tempfile,time
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('--binary',required=True);p.add_argument('--evidence',required=True);a=p.parse_args()
spec=importlib.util.spec_from_file_location('spike',Path(__file__).with_name('spike.py'));spike=importlib.util.module_from_spec(spec);spec.loader.exec_module(spike)
root=Path(tempfile.mkdtemp(prefix='apps-',dir=a.evidence));root.chmod(0o700);sock=root/'control.sock';binary=Path(a.binary).resolve();env=dict(os.environ,XDG_CONFIG_HOME=str(root/'config'),LESSHISTFILE='/dev/null');log=(root/'daemon.log').open('wb');server=subprocess.Popen([str(binary),'--socket',str(sock),'serve'],env=env,stdout=log,stderr=log);tui=None;results={}
def call(m,p=None):
 with spike.Client(sock) as c:return c.call(m,p)
def capture(pane):return call('pane.capture',{'pane':pane,'lines':40})['text']
try:
 spike.eventually(lambda:sock.exists())
 file=root/'vim.txt';s=call('session.create',{'name':'vim-fixture','argv':['vim','-u','NONE','-i','NONE','-n','--',str(file)]});tui=spike.Tui(binary,sock,s['session'],root/'vim.pty');spike.eventually(lambda:b'\x1b[H\x1b[2J' in tui.data);time.sleep(.3)
 payload='VIM_LITERAL $HOME \"quotes\" žółć 🧪'
 call('pane.sendText',{'pane':s['pane'],'text':'i'+payload});call('pane.sendKey',{'pane':s['pane'],'key':'Escape'});call('pane.sendText',{'pane':s['pane'],'text':':wq'});call('pane.sendKey',{'pane':s['pane'],'key':'Enter'});spike.eventually(lambda:file.exists() and file.read_text()==payload+'\n');tui.finish('term');tui=None;results['vim']={'status':'PASS','saved_exact_utf8':True,'tui_termios_restored':True}
 fixture=root/'less.txt';fixture.write_text('\n'.join('LESS_READY_'+str(i) for i in range(100))+'\n');s=call('session.create',{'name':'less-fixture','argv':['less','-R','--',str(fixture)]});call('pane.wait',{'pane':s['pane'],'contains':'LESS_READY_0','timeout_ms':5000});call('pane.sendText',{'pane':s['pane'],'text':'q'});results['less']={'status':'PASS','first_page_rendered':True,'quit_sent':True}
 s=call('session.create',{'name':'htop-fixture','argv':['htop','-C']});spike.eventually(lambda:'Help' in capture(s['pane']));call('pane.sendText',{'pane':s['pane'],'text':'q'});results['htop']={'status':'PASS','application_ui_rendered':True,'quit_sent':True}
 (root/'receipt.json').write_text(json.dumps({'results':results,'scope':'synthetic PTY and canonical render; not physical desktop'},indent=2)+'\n');print('APPS_ACCEPTANCE',root)
finally:
 if tui:tui.finish('term')
 if server.poll() is None:
  try:call('daemon.stop')
  except Exception:server.terminate()
  server.wait(timeout=8)
 log.close()
