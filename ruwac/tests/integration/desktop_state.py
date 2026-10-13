#!/usr/bin/env python3
"""Owned real-daemon tests for persistent layout, coherent history and mouse protocols."""
import argparse,hashlib,json,os,subprocess,sys,tempfile
from pathlib import Path
from spike import Client,eventually
p=argparse.ArgumentParser();p.add_argument('--binary',required=True);p.add_argument('--evidence',required=True);a=p.parse_args()
root=Path(tempfile.mkdtemp(prefix='desktop-state-',dir=a.evidence));root.chmod(0o700);sock=root/'control.sock';binary=Path(a.binary).resolve();log=(root/'daemon.log').open('wb');server=subprocess.Popen([str(binary),'--socket',str(sock),'serve'],stdout=log,stderr=log);results=[]
def rpc(m,p=None,**kw):
 with Client(sock) as c:return c.call(m,p,**kw)
try:
 eventually(lambda:sock.exists());receiver=Path(__file__).parents[1]/'fixtures/receiver.py';s=rpc('session.create',{'name':'desktop-state','argv':[sys.executable,str(receiver),str(root/'received.bin')],'rows':30,'cols':100});rpc('pane.wait',{'pane':s['pane'],'contains':'READY','timeout_ms':5000})
 with Client(sock) as c:
  c.call('session.attach',{'session':s['session'],'rows':30,'cols':100});one=rpc('pane.create',{'window':s['window'],'orientation':'vertical','pane':s['pane']});two=rpc('pane.create',{'window':s['window'],'orientation':'horizontal','pane':s['pane']});f=c.call('client.render');split=f['layout']['id'];rpc('pane.layout',{'window':s['window'],'split':split,'ratio':650});f=c.call('client.render');assert f['layout']['ratio']==650 and len(f['panes'])==3,f;layout=f['layout'];c.call('pane.layout',{'window':s['window'],'zoom':True});assert len(c.call('client.render')['panes'])==1;c.call('pane.layout',{'window':s['window'],'zoom':False});c.call('session.detach')
 with Client(sock) as c:
  c.call('session.attach',{'session':s['session'],'rows':30,'cols':100});f=c.call('client.render');assert f['layout']==layout and len(f['panes'])==3;results.append({'id':'layout-reconnect-ratio-zoom','status':'PASS'})
  h=rpc('pane.history',{'pane':s['pane'],'lines':1});rpc('pane.sendText',{'pane':s['pane'],'text':'epoch-change'});eventually(lambda:rpc('pane.history',{'pane':s['pane']})['epoch']!=h['epoch']);rpc('pane.history',{'pane':s['pane'],'epoch':h['epoch']},error='STALE_HISTORY');results.append({'id':'coherent-history-stale-rejection','status':'PASS'})
  def mode(command,sgr,flag):
   rpc('pane.sendText',{'pane':s['pane'],'text':command});eventually(lambda:next(g['grid'] for g in c.call('client.render')['panes'] if g['id']==s['pane']).get(flag)==True and next(g['grid'] for g in c.call('client.render')['panes'] if g['id']==s['pane']).get('mouse_sgr')==sgr)
  def mouse(button,expected):
   file=root/'received.bin';before=file.read_bytes();c.call('client.input',{'pane':s['pane'],'kind':'mouse','text':'','mouse_event':{'button':button,'x':2,'y':1,'release':False}});eventually(lambda:file.read_bytes()==before+expected)
  mode('/mouse-on/',True,'mouse');mouse(4,b'\x1b[<4;2;1M');mode('/mouse-legacy/',False,'mouse_drag');mouse(0,b'\x1b[M'+bytes([32,34,33]));mode('/mouse-motion/',True,'mouse_motion');mouse(35,b'\x1b[<35;2;1M');rpc('pane.sendText',{'pane':s['pane'],'text':'/mouse-off/'});eventually(lambda:not next(g['grid'] for g in c.call('client.render')['panes'] if g['id']==s['pane'])['mouse']);file=root/'received.bin';before=file.read_bytes();c.call('client.input',{'pane':s['pane'],'kind':'mouse','text':'','mouse_event':{'button':0,'x':2,'y':1,'release':False}});import time;time.sleep(.1);assert file.read_bytes()==before;results.append({'id':'sgr-modifier-x10-motion-disabled','status':'PASS'})
finally:
 if server.poll() is None:
  rpc('daemon.stop');server.wait(timeout=8)
 log.close();(root/'acceptance.json').write_text(json.dumps({'results':results,'binary_sha256':hashlib.sha256(binary.read_bytes()).hexdigest()},indent=2)+'\n');print('EVIDENCE',root)
assert len(results)==3
