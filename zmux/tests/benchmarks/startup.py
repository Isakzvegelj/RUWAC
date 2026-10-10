#!/usr/bin/env python3
"""Separate ready-state startup timings, using only owned private sockets."""
import argparse,importlib.util,json,os,subprocess,tempfile,time
from pathlib import Path
spec=importlib.util.spec_from_file_location('api',Path(__file__).with_name('api.py'));api=importlib.util.module_from_spec(spec);spec.loader.exec_module(api)
def run(a):
 root=Path(tempfile.mkdtemp(prefix='startup-',dir=a.evidence));os.chmod(root,0o700);values={'zMux':[],'tmux':[]};clients=[];binary=str(Path(a.binary).resolve());log=(root/'daemon.log').open('wb')
 for i in range(30):
  sock=root/f'z{i}.sock';t=time.perf_counter_ns();p=subprocess.Popen([binary,'--socket',str(sock),'serve'],stdin=subprocess.DEVNULL,stdout=log,stderr=log)
  try:
   def health():
    if not sock.exists():return False
    with api.spike.Client(sock) as c:return c.call('health')['ok']
   api.spike.eventually(health);values['zMux'].append((time.perf_counter_ns()-t)/1e6)
   with api.spike.Client(sock) as c:s=c.call('session.create',{'name':'startup','argv':['/bin/cat']})
   t=time.perf_counter_ns();view=api.spike.Tui(Path(binary),sock,s['session'],root/f'client-{i}.pty');api.spike.eventually(lambda:b'\x1b[H\x1b[2J' in view.data);clients.append((time.perf_counter_ns()-t)/1e6);view.finish('term')
  finally:
   if p.poll() is None:
    try:
     with api.spike.Client(sock) as c:c.call('daemon.stop')
    except Exception:p.terminate()
    p.wait(timeout=8)
  sock=root/f't{i}.sock';t=time.perf_counter_ns()
  try:
   subprocess.run(['tmux','-f','/dev/null','-S',str(sock),'new-session','-d','-s','startup','/bin/cat'],stdin=subprocess.DEVNULL,stdout=subprocess.DEVNULL,stderr=subprocess.PIPE,check=True,timeout=5);subprocess.run(['tmux','-S',str(sock),'list-sessions'],stdout=subprocess.DEVNULL,check=True,timeout=5);values['tmux'].append((time.perf_counter_ns()-t)/1e6)
  finally:subprocess.run(['tmux','-S',str(sock),'kill-server'],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL,timeout=5)
 log.close();d={'run_directory':str(root),'daemon_start_to_health':{k:api.stats(v) for k,v in values.items()},'client_spawn_to_first_grid':api.stats(clients),'observer_granularity_ms':5,'note':'zMux health only; tmux new-session creates a cat child then list-sessions; startup definitions differ; warm page cache, not cold filesystem'};(root/'startup.json').write_text(json.dumps(d,indent=2)+'\n');print('EVIDENCE',root)
if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('--binary',required=True);p.add_argument('--evidence',required=True);run(p.parse_args())
