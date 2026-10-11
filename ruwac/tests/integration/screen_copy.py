#!/usr/bin/env python3
"""Actual PTYs: immutable copy/search, chooser/prompt and 2–5 nested clients."""
import argparse,hashlib,json,os,signal,subprocess,sys,tempfile,time,traceback
from pathlib import Path
from spike import Client,Tui,eventually
p=argparse.ArgumentParser();p.add_argument('--binary',required=True);p.add_argument('--evidence',required=True);p.add_argument('--nested-only',action='store_true');a=p.parse_args()
binary=Path(a.binary).resolve();root=Path(tempfile.mkdtemp(prefix='screen-copy-',dir=a.evidence));root.chmod(0o700)
servers=[];tuis=[];children=[];results=[];receiver=Path(__file__).resolve().parents[1]/'fixtures/receiver.py'
def server(name,env=None):
 directory=root/name;directory.mkdir(mode=0o700);sock=directory/'control.sock'
 proc=subprocess.Popen([str(binary),'--socket',str(sock),'serve'],env=env,stdout=(directory/'daemon.log').open('wb'),stderr=subprocess.STDOUT);servers.append((proc,sock))
 eventually(lambda:sock.exists() and rpc(sock,'health')['ok']);return sock,directory
def rpc(sock,method,params=None,**kwargs):
 with Client(sock)as c:return c.call(method,params,**kwargs)
def case(name,fn):
 started=time.monotonic()
 try:details=fn();results.append({'id':name,'status':'PASS','seconds':time.monotonic()-started,'details':details});print(name,'PASS',flush=True)
 except Exception as e:results.append({'id':name,'status':'FAIL','error':str(e),'traceback':traceback.format_exc()});print(name,'FAIL',str(e),flush=True);raise

def create(sock,name,directory,script=None):
 output=directory/(name+'.bin');argv=[sys.executable,str(script or receiver),str(output)]
 row=rpc(sock,'session.create',{'name':name,'argv':argv});children.append(row['pid'])
 rpc(sock,'pane.wait',{'pane':row['pane'],'contains':'RUWAC_RECEIVER_READY','timeout_ms':10000});return row,output
try:
 if not a.nested_only:
  sock,directory=server('main')
  script=directory/'history-writer.py';script.write_text("import os,sys,tty\nfrom pathlib import Path\ntty.setraw(0)\np=Path(sys.argv[1]);p.touch(mode=0o600)\nfor i in range(9000):os.write(1,('row-%05d abcdefghijklmnopqrstuvwxyz\\r\\n'%i).encode())\nos.write(1,('SELECT:ABC界e\\u0301🧪XYZ\\r\\nRECT:abcd\\r\\nRECT:1234\\r\\nWRAP:'+('W'*90)+'END\\r\\nRUWAC_RECEIVER_READY\\r\\n').encode())\nwith p.open('ab',buffering=0)as out:\n while True:\n  b=os.read(0,8192)\n  if not b:break\n  out.write(b)\n")
  row,output=create(sock,'copy',directory,script);t=Tui(binary,sock,row['session'],directory/'copy.pty');tuis.append(t);eventually(lambda:b'RUWAC' in t.data)
  def select():
   rpc(sock,'buffer.set',{'session':row['session'],'text':'KEEP-OLD-BUFFER'})
   t.send(b'\x01[g/ABC\r $\r')
   expected='ABC界e\u0301🧪XYZ';eventually(lambda:rpc(sock,'buffer.get',{'session':row['session']})['text']==expected)
   assert output.read_bytes()==b''
   t.send(b'\x01]');eventually(lambda:output.read_bytes()==expected.encode())
   return {'canonical_rows':9000,'unicode_combining_wide_emoji':True,'internal_paste_exact':True,'modal_input_leak':False}
  case('TUI-COPY-UNICODE-SEARCH',select)
  def rectangle():
   before=output.read_bytes();t.send(b'\x01[g/RECT:abcd\r5l vjlll\r');eventually(lambda:rpc(sock,'buffer.get',{'session':row['session']})['text']=='abcd\n1234')
   assert output.read_bytes()==before
   t.send(b'\x01[g/WRAP:\r $\r');eventually(lambda:rpc(sock,'buffer.get',{'session':row['session']})['text'].startswith('WRAP:'))
   # Select across the canonical physical wrap; no inserted soft-wrap newline.
   t.send(b'\x01[g/WRAP:\r j$\r');expected='WRAP:'+'W'*90+'END';eventually(lambda:rpc(sock,'buffer.get',{'session':row['session']})['text']==expected)
   assert output.read_bytes()==before
   return {'rectangle':'abcd/newline/1234','soft_wrap_joined':True}
  case('TUI-COPY-RECTANGLE-WRAP',rectangle)
  def cancel_limits():
   before=output.read_bytes();rpc(sock,'buffer.set',{'session':row['session'],'text':'PRESERVE-LIMIT'})
   start=len(t.data);t.send(b'\x01[g G$\r');eventually(lambda:b'Selection exceeds 256 KiB'in t.data[start:])
   assert rpc(sock,'buffer.get',{'session':row['session']})['text']=='PRESERVE-LIMIT'
   t.send(b'\x1b[<0;1;1M\x1b[200~\x01d\x1b[201~');time.sleep(.1);assert output.read_bytes()==before and t.p.poll()is None
   t.send(b'\x1b');eventually(lambda:b'Copy cancelled'in t.data[start:]);t.send(b'AFTER-CANCEL');eventually(lambda:output.read_bytes()==before+b'AFTER-CANCEL')
   assert rpc(sock,'buffer.get',{'session':row['session']})['text']=='PRESERVE-LIMIT'
   return {'limit_non_destructive':True,'cancel_non_destructive':True,'mouse_and_bracketed_paste_not_forwarded':True}
  case('TUI-COPY-CANCEL-LIMIT-ISOLATION',cancel_limits)
  def chooser_prompt():
   second=rpc(sock,'window.create',{'session':row['session']});children.append(second['pid'])
   t.send(b'\x01"0\r');eventually(lambda:rpc(sock,'session.list')[0]['active_window']==row['window'])
   before=output.read_bytes();t.send('\x01:rename Unicode ž ; $HOME\r'.encode());eventually(lambda:rpc(sock,'window.list',{'session':row['session']})[0]['name']=='Unicode ž ; $HOME')
   t.send(b'\x01?');eventually(lambda:b'HELP' in t.data);t.send(b'\x1b[6~\x1b[5~\r');time.sleep(.05)
   t.send(b'\x01:zoom\r');eventually(lambda:render()['zoom']is True);t.send(b'\x01z');eventually(lambda:render()['zoom']is False)
   assert output.read_bytes()==before
   t.send(b'\x01"\x1b');eventually(lambda:b'Window choice cancelled'in t.data)
   t.send(b'\x01:unknown\r');eventually(lambda:b'Unknown command'in t.data);t.send(b'\x1b');eventually(lambda:b'Command cancelled'in t.data)
   return {'typed_window_ids':True,'rename_exact_not_shell':True,'zoom_restore':True,'help_chooser_prompt_cancel':True}
  def render():
   with Client(sock)as c:c.call('session.attach',{'session':row['session'],'rows':24,'cols':80});return c.call('client.render')
  case('TUI-CHOOSER-PROMPT-ZOOM-HELP',chooser_prompt)
  t.finish();tuis.remove(t)
  def configured():
   custom,out=create(sock,'custom',directory);env={**os.environ,'RUWAC_SHORTCUTS':'{"prefix":"<Control>b"}','RUWAC_KEYMAP':'{"q":"d"}'}
   ui=Tui(binary,sock,custom['session'],directory/'custom.pty',env=env);tuis.append(ui);eventually(lambda:b'RUWAC' in ui.data)
   ui.send(b'\x01a\x02a');eventually(lambda:out.read_bytes()==b'\x01a\x02')
   ui.send(b'\x1b[200~\x02d\x1b[201~');eventually(lambda:out.read_bytes()==b'\x01a\x02\x02d');assert ui.p.poll()is None
   ui.send(b'\x02q');ui.p.wait(timeout=6);assert ui.finish()==0;tuis.remove(ui)
   invalid=Tui(binary,sock,custom['session'],directory/'invalid-config.pty',env={**env,'RUWAC_SHORTCUTS':'{"prefix":"<Alt>b"}'});tuis.append(invalid);invalid.p.wait(timeout=5)
   import termios
   assert termios.tcgetattr(invalid.slave)==invalid.old and b'\x1b[?1049h' not in invalid.data
   invalid.finish('kill');tuis.remove(invalid)
   return {'ControlB_prefix_and_remap':True,'literal_exact':True,'paste_is_literal':True,'invalid_configuration_before_raw_mode':True}
  case('TUI-PREFIX-CONFIGURATION',configured)
 for depth in range(2,6):
  def nested(depth=depth):
   nodes=[server(f'nested-{depth}-{i}')for i in range(depth)];sessions=[None]*depth
   sessions[-1],out=create(nodes[-1][0],'nested-receiver',nodes[-1][1])
   for i in range(depth-2,-1,-1):
    sessions[i]=rpc(nodes[i][0],'session.create',{'name':f'nested layer {i}','argv':[str(binary),'--socket',str(nodes[i+1][0]),'attach','--session',sessions[i+1]['session']]});children.append(sessions[i]['pid'])
   ui=Tui(binary,nodes[0][0],sessions[0]['session'],nodes[0][1]/'outer.pty');tuis.append(ui)
   eventually(lambda:'RUWAC_RECEIVER_READY'in rpc(nodes[0][0],'pane.capture',{'pane':sessions[0]['pane']})['text'],15)
   ui.send(b'\x01'+b'a'*depth);eventually(lambda:out.read_bytes()==b'\x01',10)
   ui.send(b'\x01'+b'a'*(depth-1)+b'c');eventually(lambda:len(rpc(nodes[-1][0],'window.list',{'session':sessions[-1]['session']}))==2,10)
   ui.send(b'\x01'+b'a'*(depth-1)+b'0');eventually(lambda:rpc(nodes[-1][0],'session.list')[0]['active_window']==sessions[-1]['window'],10)
   ui.send(b'/paste-on/');eventually(lambda:out.read_bytes()==b'\x01/paste-on/',10);time.sleep(.25)
   payload='nested Živjo 🧪 '.encode()+b'\x01d';ui.send(b'\x1b[200~'+payload+b'\x1b[201~')
   eventually(lambda:out.read_bytes()==b'\x01/paste-on/\x1b[200~'+payload+b'\x1b[201~',10)
   pid=sessions[-1]['pid'];ui.finish();tuis.remove(ui)
   assert next(p for p in rpc(nodes[-1][0],'pane.list',{'session':sessions[-1]['session']})if p['id']==sessions[-1]['pane'])['pid']==pid
   ui=Tui(binary,nodes[0][0],sessions[0]['session'],nodes[0][1]/'reattach.pty');tuis.append(ui);eventually(lambda:b'RUWAC'in ui.data)
   ui.send(b'REATTACH');eventually(lambda:out.read_bytes().endswith(b'REATTACH'),10);ui.finish();tuis.remove(ui)
   return {'real_RUWAC_layers':depth,'literal_meta_prefix':True,'deep_window_selection':True,'single_bracketed_paste_pair':True,'same_receiver_pid_after_detach_reattach':pid,'termios_restored':True}
  case(f'TUI-NESTED-{depth}',nested)
finally:
 for ui in reversed(tuis):
  try:ui.finish('term')
  except Exception:
   if ui.p.poll()is None:ui.p.kill();ui.p.wait()
 for proc,sock in reversed(servers):
  if proc.poll()is None:
   try:rpc(sock,'daemon.stop');proc.wait(timeout=5)
   except Exception:proc.kill();proc.wait()
 for pid in children:
  try:
   if os.getsid(pid)==pid:os.killpg(pid,signal.SIGKILL)
  except ProcessLookupError:pass
 receipt={'results':results,'binary_sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),'evidence':str(root)}
 (root/'receipt.json').write_text(json.dumps(receipt,indent=2)+'\n');print(json.dumps(receipt,indent=2))
assert len(results)==(4 if a.nested_only else 9)
