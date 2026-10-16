#!/usr/bin/env python3
"""Real-daemon acceptance; all processes and files are evaluation-owned."""
import argparse,concurrent.futures,fcntl,hashlib,json,os,pty,select,signal,socket,stat,struct,subprocess,sys,tempfile,threading,time,traceback,uuid,zlib
from pathlib import Path

def exact(s,n):
 b=b''
 while len(b)<n:
  x=s.recv(n-len(b))
  if not x:raise EOFError('socket EOF')
  b+=x
 return b
class Client:
 def __init__(self,path):self.s=socket.socket(socket.AF_UNIX);self.s.settimeout(65);self.s.connect(str(path))
 def call(self,m,p=None,error=None,key=None):
  r={'version':0,'id':uuid.uuid4().hex,'method':m,'params':p or {}}
  if key:r['idempotency_key']=key
  b=json.dumps(r,ensure_ascii=False).encode();self.s.sendall(struct.pack('!I',len(b))+b)
  v=json.loads(exact(self.s,struct.unpack('!I',exact(self.s,4))[0]))
  assert v['id']==r['id'],v
  if error:assert v['error']['code']==error,v;return v['error']
  assert 'error' not in v,v
  return v['result']
 def close(self):self.s.close()
 def __enter__(self):return self
 def __exit__(self,*a):self.close()
def eventually(fn,timeout=5):
 end=time.monotonic()+timeout
 while time.monotonic()<end:
  try:
   v=fn()
   if v:return v
  except (OSError,EOFError):pass
  time.sleep(.005)
 raise AssertionError('condition not reached')
class Tui:
 def __init__(self,binary,sock,session,path,rows=24,cols=80,env=None,command=None):
  self.master,slave=pty.openpty();fcntl.ioctl(slave,0x5414,struct.pack('HHHH',rows,cols,0,0));self.old=__import__('termios').tcgetattr(slave);self.slave=slave
  self.p=subprocess.Popen([str(binary),'--socket',str(sock),*(command if command is not None else ['attach','--session',session])],stdin=slave,stdout=slave,stderr=slave,start_new_session=True,env=env)
  self.data=bytearray();self.stop=False;self.path=path
  def drain():
   while not self.stop:
    if select.select([self.master],[],[],.05)[0]:
     try:b=os.read(self.master,65536)
     except OSError:break
     self.data.extend(b)
  self.thread=threading.Thread(target=drain,daemon=True);self.thread.start()
 def send(self,b):os.write(self.master,b)
 def finish(self,how='detach'):
  if self.p.poll() is None:
   if how=='detach':self.send(b'\x01d')
   else:self.p.send_signal(signal.SIGTERM if how=='term' else signal.SIGKILL)
   self.p.wait(timeout=6)
  time.sleep(.03);self.stop=True;self.thread.join(timeout=1);self.path.write_bytes(self.data)
  restored=__import__('termios').tcgetattr(self.slave)==self.old
  os.close(self.master);os.close(self.slave)
  if how!='kill':assert restored,'termios not restored';assert b'\x1b[?1006l' in self.data,'no mouse reset'
  return self.p.returncode

def run(args):
 root=Path(tempfile.mkdtemp(prefix='spike-',dir=args.evidence));os.chmod(root,0o700);sock=root/'control.sock';binary=Path(args.binary).resolve();receiver=Path(__file__).resolve().parents[1]/'fixtures/receiver.py'
 log=(root/'daemon.log').open('wb');server=subprocess.Popen([str(binary),'--socket',str(sock),'serve'],stdout=log,stderr=log)
 results=[];live=[]
 def rpc(m,p=None,**kw):
  with Client(sock) as c:return c.call(m,p,**kw)
 def case(name,fn):
  t=time.monotonic()
  try:details=fn();results.append({'id':name,'status':'PASS','seconds':time.monotonic()-t,'details':details});print(name,'PASS',flush=True)
  except Exception as e:results.append({'id':name,'status':'FAIL','error':str(e),'traceback':traceback.format_exc()});print(name,'FAIL',e,flush=True);raise
 def create(name):
  path=root/(name+'.bin');out=rpc('session.create',{'name':name,'argv':[sys.executable,str(receiver),str(path)]});rpc('pane.wait',{'pane':out['pane'],'contains':'RUWAC_RECEIVER_READY','timeout_ms':5000});return out,path
 try:
  eventually(lambda:sock.exists() and rpc('health')['ok']);s,path=create('literal');sid=s['session'];pane=s['pane'];pid=s['pid']
  def literal():
   payload='$HOME "double" \'single\' \\slash\tŽivjo łódź 🧪 e\u0301\nsecond line --target=other'
   cli=subprocess.run([str(binary),'--socket',str(sock),'request'],input=json.dumps({'version':0,'id':'literal-cli','method':'pane.sendText','params':{'pane':pane,'text':payload}},ensure_ascii=False).encode(),capture_output=True,timeout=5);assert cli.returncode==0,cli.stderr
   rpc('pane.sendKey',{'pane':pane,'key':'Enter'});expected=payload.encode()+b'\r';eventually(lambda:path.read_bytes()==expected)
   rpc('pane.sendText',{'pane':pane,'text':'bad\0'},error='INVALID_TEXT');rpc('pane.sendText',{'pane':pane,'text':'x'*(256*1024+1)},error='LIMIT');rpc('pane.sendText',{'pane':'window_'+uuid.uuid4().hex,'text':'wrong'},error='INVALID_ID');rpc('pane.sendText',{'pane':'pane_'+uuid.uuid4().hex,'text':'wrong'},error='STALE_ID')
   return {'expected_hex':expected.hex(),'actual_hex':path.read_bytes().hex(),'cli_exit':cli.returncode}
  case('C01_LITERAL_CLI',literal)
  def races():
   before=path.read_bytes();samples=[]
   for i in range(100):
    t=Tui(binary,sock,sid,root/f'startup-{i:03}.pty');live.append(t)
    delay=[0,.001,.002,.005][i%4];time.sleep(delay);early=f'EARLY_{i:03}_'.encode();before_grid=b'\x1b[H\x1b[2J' not in t.data;t.send(early)
    time.sleep(.012);t.send(b'\x1b[?1;2c\x1b]11;rgb:0000/0000/0000\x07'+f'LATE_{i:03}_'.encode())
    before+=early+f'LATE_{i:03}_'.encode();eventually(lambda:path.read_bytes()==before);t.finish('term' if i%10==0 else 'detach');live.remove(t);assert rpc('pane.list',{'session':sid})[0]['pid']==pid;os.kill(pid,0)
    samples.append({'iteration':i,'injection_delay_ms':delay*1000,'receipt_bytes':len(before),'early_before_first_grid':before_grid})
   return {'runs':100,'exact':True,'samples':samples,'receipt_sha256':hashlib.sha256(before).hexdigest(),'note':'injected at 0/1/2/5 ms after process spawn; delayed DA/OSC replies excluded; dedicated PTY transcripts'}
  case('C02_STARTUP_100',races)
  def multi():
   c1=Client(sock);c2=Client(sock)
   try:
    c1.call('session.attach',{'session':sid,'rows':30,'cols':100});c2.call('session.attach',{'session':sid,'rows':20,'cols':70});assert rpc('pane.list',{'session':sid})[0]['rows']==20
    second=root/'second.bin';w=rpc('window.create',{'session':sid,'name':'two','argv':[sys.executable,str(receiver),str(second)]});rpc('pane.wait',{'pane':w['pane'],'contains':'READY','timeout_ms':5000})
    assert c1.call('client.render')['window']==c2.call('client.render')['window']==s['window'];c1.call('window.select',{'window':w['window']});assert c2.call('client.render')['window']==s['window'];before=path.read_bytes();rpc('pane.sendText',{'pane':pane,'text':'FOCUS_STABLE'});eventually(lambda:path.read_bytes()==before+b'FOCUS_STABLE');assert second.read_bytes()==b''
    splitpath=root/'split.bin';split=rpc('pane.create',{'window':w['window'],'argv':[sys.executable,str(receiver),str(splitpath)]});rpc('pane.wait',{'pane':split['pane'],'contains':'READY','timeout_ms':5000});grids=c1.call('client.render')['panes'];assert len(grids)==2;assert sum(p['grid']['rows'] for p in grids)==19
    c1.call('session.detach');assert c2.call('client.render')['session']==sid
    c1.call('session.attach',{'session':sid,'rows':24,'cols':80});assert rpc('pane.list',{'session':sid})[0]['pid']==pid
    rpc('pane.close',{'pane':split['pane']});rpc('pane.sendText',{'pane':split['pane'],'text':'stale'},error='STALE_ID')
    return {'stable_pid':pid,'window2':w,'split':split,'independent_client_focus':True,'minimum_size':[20,70],'detach_local':True}
   finally:c1.close();c2.close()
  case('C03_MULTI_FOCUS_SPLIT',multi)
  def concurrent_requests():
   before=path.read_bytes();chunks=[f'WRITE_{i}_Ž'.encode() for i in range(5)]
   with concurrent.futures.ThreadPoolExecutor(max_workers=5) as pool:list(pool.map(lambda b:rpc('pane.sendText',{'pane':pane,'text':b.decode()}),chunks))
   eventually(lambda:len(path.read_bytes())==len(before)+sum(map(len,chunks)));tail=path.read_bytes()[len(before):];assert all(tail.count(b)==1 for b in chunks)
   key=uuid.uuid4().hex;v={'pane':pane,'text':'ONCE'};rpc('pane.sendText',v,key=key);rpc('pane.sendText',v,key=key);rpc('pane.sendText',{'pane':pane,'text':'TWICE'},key=key,error='IDEMPOTENCY_CONFLICT');eventually(lambda:path.read_bytes().endswith(b'ONCE'));assert len(path.read_bytes())==len(before)+len(tail)+4
   return {'unordered_exact_chunks_hex':tail.hex(),'idempotent_single_write':True}
  case('C04_CONCURRENT_IDEMPOTENCY',concurrent_requests)
  def mousepaste():
   rpc('window.select',{'window':s['window']});c=Client(sock)
   try:
    c.call('session.attach',{'session':sid});before=path.read_bytes();mouse='\x1b[<0;1;2M';c.call('client.input',{'pane':pane,'kind':'mouse','text':mouse});time.sleep(.03);assert path.read_bytes()==before
    rpc('pane.sendText',{'pane':pane,'text':'/mouse-on/'});eventually(lambda:c.call('client.render')['panes'][0]['grid']['mouse']);before=path.read_bytes();c.call('client.input',{'pane':pane,'kind':'mouse','text':mouse});eventually(lambda:path.read_bytes()==before+mouse.encode())
    rpc('pane.sendText',{'pane':pane,'text':'/mouse-off/'});eventually(lambda:not c.call('client.render')['panes'][0]['grid']['mouse']);before=path.read_bytes();c.call('client.input',{'pane':pane,'kind':'mouse','text':mouse});time.sleep(.03);assert path.read_bytes()==before
    rpc('pane.sendText',{'pane':pane,'text':'/paste-on/'});eventually(lambda:c.call('client.render')['panes'][0]['grid']['bracketed_paste']);before=path.read_bytes();t=Tui(binary,sock,sid,root/'paste.pty');live.append(t);eventually(lambda:b'\x1b[H\x1b[2J' in t.data);t.send(b'\x1b[200~literal\x01d\t\n\x1b[201~');expected=b'\x1b[200~literal\x01d\t\n\x1b[201~';eventually(lambda:path.read_bytes()==before+expected);assert t.p.poll() is None;t.finish('term');live.remove(t)
    rpc('buffer.set',{'session':sid,'text':'INTERNAL_A'});external_simulated='EXTERNAL_B';assert rpc('buffer.get',{'session':sid})['text']=='INTERNAL_A' and external_simulated=='EXTERNAL_B'
    return {'mouse_enabled_forwarded':True,'disabled_discarded':True,'paste_hex':expected.hex(),'internal_separate_synthetic_only':True}
   finally:c.close()
  case('C05_MOUSE_PASTE_BUFFER',mousepaste)
  def lifecycle():
   identities=[]
   for how in ['detach','term','kill']:
    t=Tui(binary,sock,sid,root/f'lifecycle-{how}.pty');live.append(t);eventually(lambda:b'\x1b[H\x1b[2J' in t.data);t.finish(how);live.remove(t);os.kill(pid,0);identities.append(rpc('pane.list',{'session':sid})[0]['pid'])
   assert identities==[pid]*3
   return {'detach_sigterm_sigkill_preserve_pid':identities,'sigkill_terminal_restore':'cannot guarantee; process cannot run cleanup'}
  case('C06_LIFECYCLE',lifecycle)
  def security():
   assert stat.S_IMODE(sock.stat().st_mode)==0o600 and sock.stat().st_uid==os.getuid()
   another=subprocess.run([str(binary),'--socket',str(sock),'serve'],capture_output=True,timeout=5);assert another.returncode!=0;assert rpc('health')['ok']
   for n in [0,1024*1024+1]:
    q=socket.socket(socket.AF_UNIX);q.settimeout(2);q.connect(str(sock));q.sendall(struct.pack('!I',n));assert q.recv(1)==b'';q.close()
   q=socket.socket(socket.AF_UNIX);q.connect(str(sock));b=b'{malformed';q.sendall(struct.pack('!I',len(b))+b);bad=json.loads(exact(q,struct.unpack('!I',exact(q,4))[0]));assert bad['error']['code']=='INVALID_REQUEST';q.close()
   # Inspect only this daemon's socket descriptors and the kernel's TCP tables.
   inodes=[]
   for fd in Path(f'/proc/{server.pid}/fd').iterdir():
    try:v=os.readlink(fd)
    except FileNotFoundError:continue
    if v.startswith('socket:['):inodes.append(v[8:-1])
   for table in ['tcp','tcp6']:
    rows=Path(f'/proc/{server.pid}/net/{table}').read_text().splitlines()[1:]
    assert not any(row.split()[9] in inodes and row.split()[3]=='0A' for row in rows)
   return {'socket_mode':'0600','same_uid':sock.stat().st_uid,'duplicate_start_rejected':True,'bad_frames_rejected':True,'owned_tcp_listeners':0,'cross_uid':'source policy; separate UID test unverified'}
  case('C07_SOCKET_FRAMES_LOCK',security)
  def eventswait():
   c=Client(sock)
   try:
    c.call('events.subscribe');rpc('buffer.set',{'session':sid,'text':'EVENT'});e=json.loads(exact(c.s,struct.unpack('!I',exact(c.s,4))[0]));assert e['event']=='state.changed'
    marker='WAIT_'+uuid.uuid4().hex;hexmarker=marker.encode().hex();pool=concurrent.futures.ThreadPoolExecutor(max_workers=1);future=pool.submit(rpc,'pane.wait',{'pane':pane,'contains':hexmarker,'timeout_ms':2000});time.sleep(.03);rpc('pane.sendText',{'pane':pane,'text':marker});out=future.result(timeout=4);pool.shutdown();assert out['matched'];rpc('pane.wait',{'pane':pane,'contains':'never-'+uuid.uuid4().hex,'timeout_ms':20},error='TIMEOUT');return {'event':e,'wait_matched':True,'timeout_error':True}
   finally:c.close()
  case('C08_EVENTS_WAIT',eventswait)
  def images():
   def chunk(k,b):return struct.pack('!I',len(b))+k+b+struct.pack('!I',zlib.crc32(k+b)&0xffffffff)
   png=b'\x89PNG\r\n\x1a\n'+chunk(b'IHDR',struct.pack('!IIBBBBB',2,2,8,2,0,0,0))+chunk(b'IDAT',zlib.compress((b'\0'+b'\xff\0\0'*2)*2))+chunk(b'IEND',b'');src=root/'dummy.png';src.write_bytes(png);stage=root/'stage';stage.mkdir(mode=0o700)
   out=subprocess.run([str(binary),'image-stage',str(src),str(stage)],capture_output=True,timeout=5);assert out.returncode==0,out.stderr;r=json.loads(out.stdout);p=Path(r['path']);assert p.read_bytes()==png and r['sha256']==hashlib.sha256(png).hexdigest();assert stat.S_IMODE(p.stat().st_mode)==0o600 and r['attached'] is False
   bad=root/'not-png';bad.write_bytes(b'NOT PNG');assert subprocess.run([str(binary),'image-stage',str(bad),str(stage)],capture_output=True).returncode!=0;p.unlink();return {**r,'owned_stage_cleaned':True}
  case('C09_PNG_STAGE',images)
  def screen_prefix():
   rpc('window.select',{'window':s['window']});before=path.read_bytes();t=Tui(binary,sock,sid,root/'screen-prefix.pty');live.append(t);eventually(lambda:b'\x1b[H\x1b[2J' in t.data)
   t.send(b'\x01a\x01a');eventually(lambda:path.read_bytes()==before+b'\x01\x01')
   t.send(b'\x01n');eventually(lambda:rpc('session.list')[0]['active_window']!=s['window']);t.send(b'\x01p');eventually(lambda:rpc('session.list')[0]['active_window']==s['window'])
   count=len(rpc('window.list',{'session':sid}));t.send(b'\x01c');eventually(lambda:len(rpc('window.list',{'session':sid}))==count+1);t.finish('detach');live.remove(t);os.kill(pid,0)
   return {'create_next_previous_detach':True,'literal_ctrl_a_hex':'0101','stable_receiver_pid':pid}
  case('C11_SCREEN_PREFIX',screen_prefix)
  def stop_without_reader():
   q=socket.socket(socket.AF_UNIX);q.connect(str(sock));b=json.dumps({'version':0,'id':'stop-no-reader','method':'daemon.stop','params':{}}).encode();q.sendall(struct.pack('!I',len(b))+b);q.close();server.wait(timeout=8);assert server.returncode==0;return {'disconnected_before_ack':True,'daemon_exit':server.returncode}
  case('C10_STOP_WITHOUT_READER',stop_without_reader)
 finally:
  for t in live:
   try:t.finish('term')
   except Exception:pass
  if server.poll() is None:
   try:rpc('daemon.stop')
   except Exception:server.terminate()
   try:server.wait(timeout=8)
   except subprocess.TimeoutExpired:server.kill();server.wait()
  log.close();report={'run_directory':str(root),'binary':str(binary),'binary_sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),'server_exit':server.returncode,'results':results,'desktop':'BLOCKED','second_host':'BLOCKED'};(root/'acceptance.json').write_text(json.dumps(report,indent=2)+'\n');print('EVIDENCE',root,flush=True)
 if len(results)!=11 or any(r['status']!='PASS' for r in results):raise RuntimeError('acceptance incomplete')
if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('--binary',required=True);p.add_argument('--evidence',required=True);run(p.parse_args())
