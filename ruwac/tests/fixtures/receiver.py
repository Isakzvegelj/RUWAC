#!/usr/bin/env python3
"""Synthetic non-executing raw-byte receiver and visible echo fixture."""
import os,sys,tty
from pathlib import Path
tty.setraw(0)
p=Path(sys.argv[1]);p.touch(mode=0o600)
os.write(1,b'RUWAC_RECEIVER_READY\r\n')
with p.open('ab',buffering=0) as out:
 while True:
  try:b=os.read(0,8192)
  except OSError:break
  if not b:break
  out.write(b)
  if b'/mouse-on/' in b:os.write(1,b'\x1b[?1000h\x1b[?1006h')
  if b'/mouse-legacy/' in b:os.write(1,b'\x1b[?1006l\x1b[?1002h')
  if b'/mouse-motion/' in b:os.write(1,b'\x1b[?1003h\x1b[?1006h')
  if b'/mouse-off/' in b:os.write(1,b'\x1b[?1000l\x1b[?1002l\x1b[?1003l\x1b[?1006l')
  if b'/paste-on/' in b:os.write(1,b'\x1b[?2004h')
  os.write(1,b'SEEN:'+b.hex().encode()+b'\r\n')
