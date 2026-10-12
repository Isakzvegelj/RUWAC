#!/usr/bin/env python3
"""30-sample local CLI RTT vs isolated tmux, with separately measured render latency."""
import argparse,hashlib,importlib.util,json,math,os,resource,shutil,statistics,subprocess,sys,tempfile,time
from pathlib import Path
spec=importlib.util.spec_from_file_location('spike',Path(__file__).resolve().parents[1]/'integration/spike.py');spike=importlib.util.module_from_spec(spec);spec.loader.exec_module(spike)
def stats(samples):return {'n':len(samples),'median_ms':statistics.median(samples),'p95_ms':sorted(samples)[math.ceil(.95*len(samples))-1],'samples_ms':samples}
def cpu():
 a=[int(x) for x in Path('/proc/stat').read_text().splitlines()[0].split()[1:9]];return sum(a),a[3]+a[4]
def idle():
 a=cpu();time.sleep(1);b=cpu();return {'busy_fraction':1-(b[1]-a[1])/(b[0]-a[0]),'loadavg':Path('/proc/loadavg').read_text().strip()}
def rss(pid):
 p=Path(f'/proc/{pid}/status');s=p.read_text();return int(next(l.split()[1] for l in s.splitlines() if l.startswith('VmRSS:')))/1024
def ticks(pid):
 a=Path(f'/proc/{pid}/stat').read_text().rsplit(')',1)[1].split();return int(a[11])+int(a[12])
def own_cpu(pids):
 # Reaped CLI processes contribute through getrusage. Running fixture daemons
 # and PTY children are sampled separately; unrelated desktop activity is not
 # attributed to this benchmark. No argv or environment values are read.
 total=sum(r.ru_utime+r.ru_stime for r in [resource.getrusage(resource.RUSAGE_SELF),resource.getrusage(resource.RUSAGE_CHILDREN)])
 for pid in pids:
  try:total+=ticks(pid)/os.sysconf('SC_CLK_TCK')
  except (OSError,ValueError):pass
 return total
def run(a):
 root=Path(tempfile.mkdtemp(prefix='benchmark-',dir=a.evidence));os.chmod(root,0o700);sock=root/'control.sock';tmuxsock=root/'tmux.sock';receiver=Path(__file__).resolve().parents[1]/'fixtures/receiver.py';binary=Path(a.binary).resolve();log=(root/'daemon.log').open('wb');server=subprocess.Popen([str(binary),'--socket',str(sock),'serve'],stdout=log,stderr=log);rows=[];environment={'idle_definition':'Before/after and every CLI block external CPU busy fraction <0.20; benchmark-owned CPU subtracted only during blocks', 'tmux_sha256':hashlib.sha256(Path(shutil.which('tmux')).read_bytes()).hexdigest(),'before':idle(),'cpus':os.cpu_count(),'kernel':os.uname().release,'rust':subprocess.check_output(['rustc','--version']).decode().strip(),'tmux':subprocess.check_output(['tmux','-V']).decode().strip(),'binary_sha256':hashlib.sha256(binary.read_bytes()).hexdigest()};tui=None
 def rpc(m,p=None):
  with spike.Client(sock) as c:return c.call(m,p)
 def tm(*args):return subprocess.check_output(['tmux','-f','/dev/null','-S',str(tmuxsock),*args],stderr=subprocess.PIPE)
 def cli(m,p):return [str(binary),'--socket',str(sock),'rpc',m,'--params',json.dumps(p,ensure_ascii=False)]
 try:
  spike.eventually(lambda:sock.exists() and rpc('health')['ok']);s=rpc('session.create',{'name':'benchmark','rows':24,'cols':80,'argv':[sys.executable,str(receiver),str(root/'ruwac-0.bin')]});rpc('pane.wait',{'pane':s['pane'],'contains':'READY','timeout_ms':5000});tm('new-session','-d','-s','bench','-x','80','-y','24',sys.executable,str(receiver),str(root/'tmux-0.bin'));tp=tm('display-message','-p','-t','bench:0','#{pane_id}').decode().strip();spike.eventually(lambda:(root/'tmux-0.bin').exists());count=1
  for n in [1,5,20,40]:
   while count<n:
    w=rpc('window.create',{'session':s['session'],'argv':[sys.executable,str(receiver),str(root/f'ruwac-{count}.bin')]});rpc('pane.wait',{'pane':w['pane'],'contains':'READY','timeout_ms':5000});tm('new-window','-d','-t','bench',sys.executable,str(receiver),str(root/f'tmux-{count}.bin'));spike.eventually(lambda:(root/f'tmux-{count}.bin').exists());count+=1
   pids={server.pid,int(tm('display-message','-p','#{pid}').decode())}|{p['pid']for p in rpc('pane.list',{'session':s['session']})if p['pid']}|{int(p)for p in tm('list-panes','-s','-t','bench','-F','#{pane_pid}').decode().split()}
   commands={
    'list':{'ruwac':cli('pane.list',{'session':s['session']}),'tmux':['tmux','-S',str(tmuxsock),'list-panes','-s','-t','bench','-F','#{pane_id} #{window_id} #{pane_pid}']},
    'send':{'ruwac':cli('pane.sendText',{'pane':s['pane'],'text':'x'}),'tmux':['tmux','-S',str(tmuxsock),'send-keys','-t',tp,'-l','--','x']},
    'capture':{'ruwac':cli('pane.capture',{'pane':s['pane'],'lines':250}),'tmux':['tmux','-S',str(tmuxsock),'capture-pane','-p','-t',tp,'-S','-250']}}
   for action,cmds in commands.items():
    samples={k:[] for k in cmds}
    for cmd in cmds.values():
     for _ in range(3):subprocess.run(cmd,stdout=subprocess.DEVNULL,stderr=subprocess.PIPE,check=True,timeout=5)
    # Alternate order to reduce drift; both include real CLI process startup.
    usage_before=own_cpu(pids);cpu_before=cpu()
    for i in range(30):
     for name in (list(cmds) if i%2==0 else list(reversed(cmds))):
      t=time.perf_counter_ns();subprocess.run(cmds[name],stdout=subprocess.DEVNULL,stderr=subprocess.PIPE,check=True,timeout=5);samples[name].append((time.perf_counter_ns()-t)/1e6)
    cpu_after=cpu();usage_after=own_cpu(pids);total=(cpu_after[0]-cpu_before[0])/os.sysconf('SC_CLK_TCK');busy=(cpu_after[0]-cpu_before[0]-(cpu_after[1]-cpu_before[1]))/os.sysconf('SC_CLK_TCK');owned=max(0,usage_after-usage_before);external=max(0,busy-owned)/max(total,.0001)
    qualification={'total_cpu_seconds':total,'busy_cpu_seconds':busy,'owned_cpu_seconds':owned,'external_busy_fraction':external,'idle_qualified':external<.20}
    z=stats(samples['ruwac']);baseline=stats(samples['tmux']);ratio=z['median_ms']/baseline['median_ms'];rows.append({'panes':n,'action':action,'ruwac':z,'tmux':baseline,'ratio':ratio,'within_2x':ratio<=2,'environment':qualification});print(n,action,round(z['median_ms'],3),round(baseline['median_ms'],3),round(ratio,2),flush=True)
  rpc('window.select',{'window':s['window']});tui=spike.Tui(binary,sock,s['session'],root/'render-latency.pty');spike.eventually(lambda:b'\x1b[H\x1b[2J' in tui.data);lat=[]
  for i in range(30):
   marker=f'K{i:03}';visible=marker.encode().hex().encode();offset=len(tui.data);start=time.perf_counter_ns();tui.send(marker.encode());spike.eventually(lambda:visible in tui.data[offset:]);lat.append((time.perf_counter_ns()-start)/1e6)
  render=stats(lat);render['method']='key bytes written to dedicated client PTY until hex echo visible in emitted canonical render; 5ms observer granularity; not physical photon latency'
  t1=ticks(server.pid);clientticks=ticks(tui.p.pid);time.sleep(1);resources={'server_rss_mib':rss(server.pid),'client_rss_mib':rss(tui.p.pid),'server_cpu_percent_one_core':(ticks(server.pid)-t1)/os.sysconf('SC_CLK_TCK')*100,'client_cpu_percent_one_core':(ticks(tui.p.pid)-clientticks)/os.sysconf('SC_CLK_TCK')*100,'panes':40,'note':'daemon and TUI only; excludes child interpreters'};tui.finish('term');tui=None
  environment['after']=idle();receipt={'run_directory':str(root),'environment':environment,'idle_qualified':environment['before']['busy_fraction']<.2 and environment['after']['busy_fraction']<.2 and all(row['environment']['idle_qualified']for row in rows),'rows':rows,'key_to_render':render,'resources':resources,'all_api_within_2x':all(r['within_2x'] for r in rows)};(root/'benchmarks.json').write_text(json.dumps(receipt,indent=2)+'\n');print('EVIDENCE',root,flush=True)
 finally:
  if tui:
   try:tui.finish('term')
   except Exception:pass
  try:tm('kill-server')
  except Exception:pass
  if server.poll() is None:
   try:rpc('daemon.stop')
   except Exception:server.terminate()
   server.wait(timeout=8)
  log.close()
if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('--binary',required=True);p.add_argument('--evidence',required=True);run(p.parse_args())
