#!/usr/bin/env python3
"""Disposable raw child / foreground recovery probe, generated input only."""
import hashlib,json,os,sys,termios,tty
from pathlib import Path
mode,path=sys.argv[1:];path=Path(path)
if mode=='raw':
 tty.setraw(0);os.write(1,b'\x1b[?1000h\x1b[?1006h\x1b[?2004hRAW-READY\r\n');path.write_text(json.dumps({'pid':os.getpid(),'sid':os.getsid(0)}))
 while True:os.read(0,1024)
else:
 flags=termios.tcgetattr(0)[3];path.write_text(json.dumps({'echo':bool(flags&termios.ECHO),'canonical':bool(flags&termios.ICANON)}));os.write(1,b'PROBE-READY\r\n');data=os.read(0,4096);path.write_text(json.dumps({'echo':bool(flags&termios.ECHO),'canonical':bool(flags&termios.ICANON),'bytes':len(data),'sha256':hashlib.sha256(data).hexdigest()}))
