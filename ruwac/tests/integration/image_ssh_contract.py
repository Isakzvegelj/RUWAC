#!/usr/bin/env python3
"""Synthetic SSH receipt/framing checks, separate from real-host acceptance."""
import argparse,json,os,struct,subprocess,tempfile,zlib
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('--binary',required=True);p.add_argument('--evidence',type=Path,required=True);a=p.parse_args()
root=Path(tempfile.mkdtemp(prefix='image-ssh-contract-',dir=a.evidence));root.chmod(0o700);bin=root/'bin';bin.mkdir()
def chunk(kind,data):return struct.pack('!I',len(data))+kind+data+struct.pack('!I',zlib.crc32(kind+data)&0xffffffff)
png=b'\x89PNG\r\n\x1a\n'+chunk(b'IHDR',struct.pack('!IIBBBBB',1,1,8,2,0,0,0))+chunk(b'IDAT',zlib.compress(b'\0\xff\0\0'))+chunk(b'IEND',b'');source=root/'source.png';source.write_bytes(png)
peer=bin/'ssh';peer.write_text('''#!/usr/bin/env python3
import hashlib,json,os,struct,sys
from pathlib import Path
raw=sys.stdin.buffer.read();args=sys.argv[1:];command=args[-1]
if 'image-receive' not in command:
 q=json.loads(raw);result={'methods':['pane.attachImage']}if q['method']=='schema'else{'foreground':{'pid':1}}
 print(json.dumps({'version':0,'id':q['id'],'result':result}));sys.exit()
size=struct.unpack('!I',raw[:4])[0];meta=json.loads(raw[4:4+size]);png=raw[4+size:];Path(os.environ['PEER_RECEIPT']).write_text(json.dumps({'args':args,'metadata':meta,'sha256':hashlib.sha256(png).hexdigest(),'bytes':len(png)}))
r={'pane':meta['pane'],'path':meta['directory']+'/'+'a'*32+'.png','sha256':meta['sha256'],'bytes':meta['bytes'],'width':meta['width'],'height':meta['height'],'file_mode':384,'directory_mode':448,'submitted':False,'consumed':False,'queued':True,'draft_attachment_observed':True,'adapter':'codex'}
v={'version':0,'id':meta['id'],'result':r};mode=os.environ['PEER_MODE']
if mode=='wrong-id':v['id']='unrelated'
if mode=='wrong-version':v['version']=9
if mode=='wrong-hash':r['sha256']='0'*64
if mode=='wrong-mode':r['file_mode']=420
if mode=='wrong-directory':r['path']='/different/'+'a'*32+'.png'
if mode=='unicode-name':r['path']=meta['directory']+'/'+'a'*31+'é.png'
if mode=='fake-consumption':r['consumed']=True
if mode=='wrong-type':r['draft_attachment_observed']='true'
print(json.dumps(v))
''');peer.chmod(0o700)
receipt=root/'peer.json';base=[a.binary,'--host','authorized-alias','--remote-binary',"remote ' ; $(literal)",'--socket',"/private/socket ' ; $(literal)",'image-attach','pane_'+'1'*32,str(source),'/private/stage'];rows=[]
for mode in ['ok','wrong-id','wrong-version','wrong-hash','wrong-mode','wrong-directory','unicode-name','fake-consumption','wrong-type']:
 r=subprocess.run(base,env={**os.environ,'PATH':str(bin)+os.pathsep+os.environ['PATH'],'TMPDIR':str(root),'PEER_MODE':mode,'PEER_RECEIPT':str(receipt)},capture_output=True,text=True,timeout=5)
 assert (r.returncode==0)==(mode=='ok'),(mode,r.stdout,r.stderr)
 d=json.loads(receipt.read_text());assert d['sha256']==d['metadata']['sha256']and d['bytes']==len(png)
 import shlex
 assert shlex.split(d['args'][-1])==["remote ' ; $(literal)",'--socket',"/private/socket ' ; $(literal)",'image-receive']
 assert '/private/stage'not in d['args'][-1], 'upload metadata entered the remote shell command'
 assert all(v in d['args']for v in ['StrictHostKeyChecking=yes','ForwardAgent=no','ClearAllForwardings=yes'])
 assert not list(root.glob('ruwac-image-upload_*')), 'local temporary upload leaked'
 rows.append({'case':mode,'PASS':True});print(mode,'PASS')
(root/'receipt.json').write_text(json.dumps({'cases':rows,'qualification':'Synthetic peer validation; no provider or real SSH consumption claim'},indent=2)+'\n')
print('IMAGE_SSH_CONTRACT PASS',root)
