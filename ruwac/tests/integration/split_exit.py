#!/usr/bin/env python3
"""Real split exit: collapse, focus, geometry, input, multi-attach and old-daemon compatibility."""
import argparse,json,os,subprocess,sys,tempfile,time
from pathlib import Path
from spike import Client,Tui,eventually
p=argparse.ArgumentParser();p.add_argument('--binary',type=Path,required=True);p.add_argument('--daemon',type=Path);p.add_argument('--gui',type=Path);p.add_argument('--evidence',type=Path,required=True);a=p.parse_args()
if a.gui and not os.environ.get('WAYLAND_DISPLAY','').startswith('ruwac-'):p.error('Owned test compositor required')
a.evidence.mkdir(parents=True,exist_ok=True);root=Path(tempfile.mkdtemp(prefix='split-exit-',dir=a.evidence));root.chmod(0o700);sock=root/'control.sock';env={**os.environ,'TERM':'xterm-256color','RUWAC_STATE_FILE':str(root/'state.json'),'GTK_A11Y':'none','GSETTINGS_BACKEND':'memory'}
for k,n in [('HOME','home'),('XDG_CONFIG_HOME','config'),('XDG_DATA_HOME','data'),('XDG_CACHE_HOME','cache')]:env[k]=str(root/n);Path(env[k]).mkdir(mode=0o700)
for k in ['RUWAC_SHORTCUTS','RUWAC_KEYMAP','ENV','BASH_ENV','TMUX','STY']:env.pop(k,None)
children=[];views=[];checks=[];timings=[];seq=0
receiver=root/'receiver.py';receiver.write_text("""import os,signal,sys,tty
from pathlib import Path
tty.setraw(0)
def resized(*_):
 size=os.get_terminal_size(0)
 if (size.lines,size.columns)==(23,80):os.write(1,b'\\r\\nFULL_AFTER_EXIT_READY\\r\\n')
signal.signal(signal.SIGWINCH,resized)
os.write(1,b'RECEIVER_READY\\r\\n');resized()
with Path(sys.argv[1]).open('ab',buffering=0)as output:
 while True:
  chunk=os.read(0,4096)
  if not chunk:break
  output.write(chunk);os.write(1,chunk)
""")


def start(args,name):
 f=(root/(name+'.log')).open('w');q=subprocess.Popen(args,env=env,stdout=f,stderr=f);children.append((q,f));return q

def rpc(method,params=None):
 with Client(sock)as c:return c.call(method,params)

def pane_ids(session):return {v['id']for v in rpc('pane.list',{'session':session})}

def gui(session,name):
 global seq
 fixture=root/name;fixture.mkdir(mode=0o700)
 child=start([str(a.gui),'--new-instance','--socket',str(sock),'--session',session,'--fixture-dir',str(fixture)],name)
 def command(action,**kw):
  global seq
  seq+=1;tmp=fixture/'request.tmp';tmp.write_text(json.dumps({'seq':seq,'action':action,**kw}));tmp.replace(fixture/'gui-request.json')
  def read():
   try:q=json.loads((fixture/'gui-response.json').read_text());return q['result']if q['seq']==seq else None
   except(OSError,ValueError):return None
  return eventually(read,12)
 eventually(lambda:command('inspect'),15)
 return child,command

def tui(session,name):
 v=Tui(a.binary,sock,session,root/(name+'.pty'),env=env);views.append(v);eventually(lambda:b'\x1b[H\x1b[2J'in v.data);return v

try:
 daemon=start([str(a.daemon or a.binary),'--socket',str(sock),'serve'],'daemon');eventually(sock.exists)
 for mode,orientation in [(m,o)for m in (['tui','gtk']if a.gui else ['tui'])for o in ['vertical','horizontal']]:
  received=root/(mode+'-'+orientation+'.bin');s=rpc('session.create',{'name':'Generated '+orientation,'rows':40,'cols':120,'argv':[sys.executable,str(receiver),str(received)]});rpc('pane.wait',{'pane':s['pane'],'contains':'RECEIVER_READY','timeout_ms':5000});dead=rpc('pane.create',{'window':s['window'],'orientation':orientation,'argv':['/bin/sh','-c','printf SPLIT_READY; exec /bin/sh']});rpc('pane.wait',{'pane':dead['pane'],'contains':'SPLIT_READY','timeout_ms':5000})
  pair=[tui(s['session'],orientation+'-a'),tui(s['session'],orientation+'-b')]if mode=='tui'else[]
  native=None
  if mode=='gtk':native,command=gui(s['session'],orientation+'-gtk')
  with Client(sock)as c:
   c.call('session.attach',{'session':s['session'],'rows':23,'cols':80});before=c.call('client.render');assert len(before['panes'])==2
   began=time.monotonic();rpc('pane.sendText',{'pane':dead['pane'],'text':'exit\r'})
   eventually(lambda:dead['pane']not in pane_ids(s['session']),6)
   timings.append({'client':mode,'orientation':orientation,'request_to_observed_layout_ms':round((time.monotonic()-began)*1000,3)})
   after=c.call('client.render');assert len(after['panes'])==1 and after['panes'][0]['id']==s['pane'];assert after['active_pane']==s['pane']
   rect=after['panes'][0]['rect'];size=next(v for v in rpc('session.list')if v['id']==s['session']);assert rect['x']==rect['y']==0 and rect['cols']==size['cols'] and rect['rows']==size['rows'],after
   # Wait for the actual child's SIGWINCH banner to reach each physical TUI.
   # API removal alone does not mean every client's next frame is visible.
   for v in pair:eventually(lambda:b'FULL_AFTER_EXIT_READY'in v.data,6)
   for i,v in enumerate(pair):
    assert v.p.poll()is None;marker='AFTER_EXIT_'+orientation+str(i);v.send((marker+'\r').encode());eventually(lambda:marker in rpc('pane.capture',{'pane':s['pane']})['text'])
   if pair:assert received.read_bytes()==b''.join(('AFTER_EXIT_'+orientation+str(i)+'\r').encode()for i in range(2))
   if native:
    eventually(lambda:command('inspect').get('target')==s['pane']);assert native.poll()is None
    for key in 'gtkafterexit':command('key',key=key,mods=0)
    command('key',key='Return',mods=0);eventually(lambda:'gtkafterexit'in rpc('pane.capture',{'pane':s['pane']})['text']);native.terminate();native.wait(timeout=8)
   live=rpc('pane.list',{'session':s['session']});assert len(live)==1 and live[0]['pid']==s['pid']and not live[0]['ended'];os.kill(s['pid'],0)
  for v in pair:assert v.finish()==0;views.remove(v)
  checks.append(mode+'-'+orientation+'-exit-collapses-and-preserves-shared-clients-focus-geometry-input-PID')
 # No client observed the exit: attaching must still repair the mixed layout.
 s=rpc('session.create',{'name':'Generated detached exit','argv':['/bin/sh','-c','printf RETIRED_READY; exec /bin/sh']});other=rpc('pane.create',{'window':s['window'],'orientation':'vertical','argv':['/bin/cat']});rpc('pane.sendText',{'pane':s['pane'],'text':'exit\r'});eventually(lambda:next(v for v in rpc('pane.list',{'session':s['session']})if v['id']==s['pane'])['ended'])
 v=tui(s['session'],'detached');eventually(lambda:pane_ids(s['session'])=={other['pane']},6);assert v.finish()==0;views.remove(v);checks.append('first-attachment-collapses-already-ended-sibling')
 # Nested split must retain both live branches, not merely zoom one pane.
 s=rpc('session.create',{'name':'Generated nested exit','argv':['/bin/cat']});other=rpc('pane.create',{'window':s['window'],'orientation':'vertical','argv':['/bin/cat']});dead=rpc('pane.create',{'window':s['window'],'orientation':'horizontal','argv':['/bin/sh','-c','printf NESTED_READY; exec /bin/sh']});rpc('pane.wait',{'pane':dead['pane'],'contains':'NESTED_READY','timeout_ms':5000});v=tui(s['session'],'nested');rpc('pane.sendText',{'pane':dead['pane'],'text':'exit\r'});eventually(lambda:pane_ids(s['session'])=={s['pane'],other['pane']},6)
 with Client(sock)as c:
  c.call('session.attach',{'session':s['session'],'rows':23,'cols':80});frame=c.call('client.render');assert len(frame['panes'])==2 and sum(q['rect']['rows']*q['rect']['cols']for q in frame['panes'])==next(q for q in rpc('session.list')if q['id']==s['session'])['rows']*next(q for q in rpc('session.list')if q['id']==s['session'])['cols']-(0 if a.daemon else frame['rows'])
  for leaf in [s,other]:c.call('pane.select',{'pane':leaf['pane']});c.call('client.input',{'pane':leaf['pane'],'kind':'keyboard','text':'NESTED_SURVIVOR\n'});eventually(lambda:'NESTED_SURVIVOR'in rpc('pane.capture',{'pane':leaf['pane']})['text']);assert next(q for q in rpc('pane.list',{'session':s['session']})if q['id']==leaf['pane'])['pid']==leaf['pid']
 assert v.finish()==0;views.remove(v);checks.append('nested-layout-removes-only-exited-leaf-and-preserves-both-live-branches')
 print('Split exit PASS',len(checks),'groups')
finally:
 for v in views:v.finish('kill')
 for child,f in reversed(children):
  if child.poll()is None:child.terminate()
  try:child.wait(timeout=8)
  except subprocess.TimeoutExpired:child.kill();child.wait()
  f.close()
 (root/'receipt.json').write_text(json.dumps({'PASS':len(checks)==(6 if a.gui else 4),'checks':checks,'timings':timings,'timing_scope':'PTY exit request to observed canonical layout via polling, not physical display latency','scope':'Actual PTYs and concurrent TUI clients; optional native GTK on owned compositor, no owner sessions touched'},indent=2)+'\n')
