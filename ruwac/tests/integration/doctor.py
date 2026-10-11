#!/usr/bin/env python3
"""Bounded read-only diagnosis and foreground-only repair on disposable PTYs."""
import argparse,fcntl,json,os,pty,socket,subprocess,tempfile,termios,time
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('--binary',required=True);p.add_argument('--evidence',type=Path,required=True);a=p.parse_args();root=Path(tempfile.mkdtemp(prefix='doctor-',dir=a.evidence));root.chmod(0o700);sock=root/'control.sock';base=[a.binary,'--socket',str(sock),'doctor'];cases=[];daemon=None

def run(args=[],ok=True,**kw):
 r=subprocess.run(base+args,capture_output=True,timeout=4,**kw);assert(r.returncode==0)==ok,(args,r.stdout,r.stderr)
 return json.loads(r.stdout)
def statuses(d):return {c['name']:c['status']for c in d['result']['checks']}
try:
 r=run();assert statuses(r)['daemon']=='WARN' and not sock.exists();cases.append({'id':'DOCTOR-NO-AUTOSTART','status':'PASS'})
 sock.write_text('preserve original');sock.chmod(0o600);assert statuses(run(ok=False))['daemon-socket']=='FAIL' and sock.read_text()=='preserve original';sock.unlink()
 target=root/'target';target.write_text('private unrelated file');sock.symlink_to(target);run(ok=False);assert sock.is_symlink() and target.read_text()=='private unrelated file';sock.unlink();cases.append({'id':'DOCTOR-UNSAFE-PATHS','status':'PASS'})
 listener=socket.socket(socket.AF_UNIX);listener.bind(str(sock));sock.chmod(0o600);listener.listen(1);started=time.monotonic();assert statuses(run(ok=False))['daemon']=='FAIL';assert time.monotonic()-started<2;listener.close();sock.unlink();cases.append({'id':'DOCTOR-BOUNDED-LISTENER','status':'PASS'})
 subprocess.run([a.binary,'--socket',str(sock),'start'],check=True,capture_output=True,timeout=8)
 def health():return json.loads(subprocess.check_output([a.binary,'--socket',str(sock),'health']))['result']
 before=health();assert statuses(run())['daemon']=='PASS';assert health()==before;cases.append({'id':'DOCTOR-LIVE-READONLY','status':'PASS'})
 run(['--terminal'],ok=False);assert not run(['--terminal'],ok=False)['result']['repair_requested']
 env={**os.environ,'RUWAC_GUI_ENTRY':str(root/'missing-desktop')};env.pop('DISPLAY',None);env.pop('WAYLAND_DISPLAY',None)
 r=run(['--desktop'],ok=False,env=env);assert statuses(r)['desktop-binary']=='FAIL' and statuses(r)['desktop-display']=='FAIL';cases.append({'id':'DOCTOR-MISSING-DESKTOP-NO-TTY','status':'PASS'})
 m,s=pty.openpty();other_m,other_s=pty.openpty();initial=termios.tcgetattr(s);raw=termios.tcgetattr(s);raw[3]&=~(termios.ICANON|termios.ECHO|termios.ISIG);termios.tcsetattr(s,termios.TCSANOW,raw);termios.tcsetattr(other_s,termios.TCSANOW,raw);sentinel=termios.tcgetattr(other_s);raw=termios.tcgetattr(s)
 def foreground():os.setsid();fcntl.ioctl(0,termios.TIOCSCTTY,0)
 r=run(['--terminal'],stdin=s);assert statuses(r)['terminal']=='WARN' and termios.tcgetattr(s)==raw
 r=run(['--terminal','--repair'],stdin=s,preexec_fn=foreground);assert statuses(r)['terminal-repair']=='PASS'
 restored=termios.tcgetattr(s);assert restored[3]&(termios.ICANON|termios.ECHO|termios.ISIG)==termios.ICANON|termios.ECHO|termios.ISIG and restored[6]==initial[6] and termios.tcgetattr(other_s)==sentinel
 termios.tcsetattr(s,termios.TCSANOW,raw)
 r=run(['--terminal','--repair'],stdin=s,ok=False);assert statuses(r)['terminal-repair']=='FAIL' and termios.tcgetattr(s)==raw
 for fd in [m,s,other_m,other_s]:os.close(fd)
 cases.append({'id':'DOCTOR-FOREGROUND-REPAIR','status':'PASS','read_only_without_repair':True,'foreground_only':True,'other_tty_unchanged':True,'custom_control_characters_preserved':True,'no_escape_sequences':True})
 for args in [['--repair'],['--terminal','--terminal'],['--unknown']]:
  r=subprocess.run(base+args,capture_output=True,timeout=4);assert r.returncode
 cases.append({'id':'DOCTOR-OPTION-VALIDATION','status':'PASS'})
finally:
 if sock.exists() and sock.is_socket():subprocess.run([a.binary,'--socket',str(sock),'rpc','daemon.stop'],capture_output=True,timeout=8)
 (root/'receipt.json').write_text(json.dumps({'cases':cases},indent=2)+'\n');(a.evidence/'doctor-acceptance.json').write_text(json.dumps({'cases':cases},indent=2)+'\n');print(json.dumps({'cases':cases,'evidence':str(root)},indent=2))
