#!/usr/bin/env python3
"""Forward real OpenSSH SFTP packets, withholding one successful rename reply for a fault test."""
import json,struct,subprocess,sys,threading
from pathlib import Path
root=Path(sys.argv[1]);backend=subprocess.Popen([sys.argv[2]],stdin=subprocess.PIPE,stdout=subprocess.PIPE);renames={}
def exact(stream,n):
 data=b''
 while len(data)<n:
  part=stream.read(n-len(data))
  if not part:raise EOFError
  data+=part
 return data
def packet(stream):
 prefix=exact(stream,4);size=struct.unpack('!I',prefix)[0]
 if not 1<=size<=2*1024*1024:raise EOFError
 return prefix,exact(stream,size)
def string(data,offset):
 size=struct.unpack('!I',data[offset:offset+4])[0];return data[offset+4:offset+4+size],offset+4+size
def input_loop():
 try:
  while True:
   prefix,data=packet(sys.stdin.buffer)
   if data[0] in [18,200]:
    offset=5
    if data[0]==200:
     extension,offset=string(data,offset)
     if extension!=b'posix-rename@openssh.com':offset=None
    if offset is not None:
     old,offset=string(data,offset);new,offset=string(data,offset)
     if Path(new.decode()).name.startswith('ruwac-backup-transfer_'):renames[data[1:5]]='backup'
     elif Path(old.decode()).name.startswith('ruwac-part-transfer_') and Path(new.decode()).name=='fault-final':renames[data[1:5]]='final'
   backend.stdin.write(prefix+data);backend.stdin.flush()
 except (EOFError,OSError,ValueError,struct.error):
  try:backend.stdin.close()
  except OSError:pass
threading.Thread(target=input_loop,daemon=True).start()
try:
 while True:
  prefix,data=packet(backend.stdout);kind=renames.pop(data[1:5],None) if data[0]==101 else None
  if kind and len(data)>=9 and struct.unpack('!I',data[5:9])[0]==0 and (root/('drop-'+kind+'-once')).exists():
   (root/('drop-'+kind+'-once')).unlink();(root/('withheld-'+kind+'.json')).write_text(json.dumps({'real_OpenSSH_rename_succeeded':True,'success_reply_withheld':True}))
   break
  sys.stdout.buffer.write(prefix+data);sys.stdout.buffer.flush()
except (EOFError,OSError):pass
finally:
 if backend.poll() is None:backend.terminate()
 backend.wait(timeout=5)
