#!/usr/bin/env python3
"""Bounded binary-transfer refusal tests; no provider or external host assumed."""
import argparse,base64,hashlib,json,os,struct,subprocess,tempfile,uuid
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('--binary',required=True);p.add_argument('--evidence',type=Path,required=True);a=p.parse_args()
root=Path(tempfile.mkdtemp(prefix='image-transport-',dir=a.evidence));root.chmod(0o700)
# A decoded 1x1 PNG, not a provider prompt or synthetic consumption acknowledgement.
png=base64.b64decode('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+aH3sAAAAASUVORK5CYII=')
stage=root/'stage';stage.mkdir(mode=0o700);source=root/'source.png';source.write_bytes(png)
# Use RUWAC's decoder as the input fixture sanity check before transport negatives.
checked=subprocess.run([a.binary,'image-stage',str(source),str(stage)],capture_output=True,text=True,timeout=3)
if checked.returncode:
    # Generate a fresh standards-compliant PNG with the standard library.
    import zlib
    def chunk(kind,data):return struct.pack('!I',len(data))+kind+data+struct.pack('!I',zlib.crc32(kind+data)&0xffffffff)
    png=b'\x89PNG\r\n\x1a\n'+chunk(b'IHDR',struct.pack('!IIBBBBB',1,1,8,2,0,0,0))+chunk(b'IDAT',zlib.compress(b'\0\xff\0\0'))+chunk(b'IEND',b'')
    source.write_bytes(png)
    checked=subprocess.run([a.binary,'image-stage',str(source),str(stage)],capture_output=True,text=True,timeout=3)
assert checked.returncode==0,checked.stderr
Path(json.loads(checked.stdout)['path']).unlink()
header={'id':'image-transfer_'+uuid.uuid4().hex,'pane':'pane_'+uuid.uuid4().hex,'directory':str(stage),'foreground_pid':1,'sha256':hashlib.sha256(png).hexdigest(),'bytes':len(png),'width':1,'height':1}
def encoded(h,b):
    h=json.dumps(h).encode();return struct.pack('!I',len(h))+h+b
cases=[('oversized-header',struct.pack('!I',8193)),('unknown-field',encoded({**header,'unknown':True},png)),('oversized-body',encoded({**header,'bytes':64*1024*1024+1},b'')),('truncated',encoded(header,png[:-1])),('extra-byte',encoded(header,png+b'x')),('wrong-hash',encoded({**header,'sha256':'0'*64},png)),('wrong-dimensions',encoded({**header,'width':2},png)),('invalid-pane',encoded({**header,'pane':'implicit'},png))]
for name,payload in cases:
    result=subprocess.run([a.binary,'--socket',str(root/'never-created.sock'),'image-receive'],input=payload,capture_output=True,timeout=3)
    assert result.returncode!=0,(name,result.stdout)
    assert not list(stage.iterdir()),(name,'partial image leaked')
    assert not(root/'never-created.sock').exists(),(name,'invalid upload contacted a daemon')
    print(name,'PASS')
stage.chmod(0o755)
result=subprocess.run([a.binary,'image-receive'],input=encoded(header,png),capture_output=True,timeout=3);assert result.returncode and not list(stage.iterdir())
(root/'receipt.json').write_text(json.dumps({'refusal_cases':9,'payload':'actual decoded PNG','provider_consumption':'not claimed','partial_upload_cleanup':True},indent=2)+'\n')
print('IMAGE_TRANSPORT_REFUSALS PASS',root)
