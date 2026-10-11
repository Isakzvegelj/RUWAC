#!/usr/bin/env python3
"""On-demand daemon startup, races and unsafe-listener rejection; isolated sockets only."""
import argparse, concurrent.futures, json, os, socket, struct, subprocess, tempfile, threading
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('--binary',required=True);p.add_argument('--evidence',required=True);a=p.parse_args()
root=Path(tempfile.mkdtemp(prefix='startup-',dir=a.evidence));root.chmod(0o700);sock=root/'daemon.sock';results=[]
def invoke(path):
 return subprocess.run([a.binary,'--socket',str(path),'start'],capture_output=True,text=True,timeout=12)
def peer(path):
 with socket.socket(socket.AF_UNIX) as s:
  s.connect(str(path));return struct.unpack('3i',s.getsockopt(socket.SOL_SOCKET,socket.SO_PEERCRED,12))[0]
try:
 with concurrent.futures.ThreadPoolExecutor(max_workers=8) as pool: runs=list(pool.map(lambda _:invoke(sock),range(8)))
 assert all(r.returncode==0 and json.loads(r.stdout)['result']['ready'] for r in runs),[(r.returncode,r.stderr)for r in runs]
 pid=peer(sock);assert invoke(sock).returncode==0 and peer(sock)==pid
 assert sock.stat().st_mode&0o777==0o600
 results.append({'id':'START-RACE-8','status':'PASS','same_daemon_pid':pid,'socket_mode':'0600'})
 bad=root/'unsafe';bad.mkdir(mode=0o755);r=invoke(bad/'daemon.sock');assert r.returncode!=0 and not (bad/'daemon.sock').exists()
 regular=root/'regular';regular.write_text('retain');r=invoke(regular);assert r.returncode!=0 and regular.read_text()=='retain'
 linked=root/'linked';linked.symlink_to(sock);r=invoke(linked);assert r.returncode!=0 and linked.is_symlink()
 results.append({'id':'START-UNSAFE-PATHS','status':'PASS'})
 unknown=root/'unknown.sock'
 with socket.socket(socket.AF_UNIX) as listener:
  listener.bind(str(unknown));unknown.chmod(0o600);listener.listen();r=invoke(unknown)
  assert r.returncode!=0 and unknown.exists()
 results.append({'id':'START-SLOW-LISTENER','status':'PASS','listener_preserved':True})
 fake=root/'mismatched.sock'
 with socket.socket(socket.AF_UNIX)as listener:
  listener.bind(str(fake));fake.chmod(0o600);listener.listen()
  def respond():
   c,_=listener.accept()
   with c:
    header=c.recv(4);size=struct.unpack('!I',header)[0];data=b''
    while len(data)<size:data+=c.recv(size-len(data))
    packet=json.dumps({'version':0,'id':'wrong-correlation','result':{'ok':True}}).encode();c.sendall(struct.pack('!I',len(packet))+packet)
  thread=threading.Thread(target=respond);thread.start();r=subprocess.run([a.binary,'--socket',str(fake),'health'],capture_output=True,text=True,timeout=5);thread.join(timeout=5);assert r.returncode!=0 and not r.stdout and fake.exists()
 results.append({'id':'CLI-CORRELATION-REJECT','status':'PASS','unrelated_listener_preserved':True})
finally:
 if sock.exists():subprocess.run([a.binary,'--socket',str(sock),'rpc','daemon.stop','--params','{}'],capture_output=True,timeout=10)
 (root/'receipt.json').write_text(json.dumps({'results':results},indent=2));print(json.dumps({'results':results,'evidence':str(root)},indent=2))
