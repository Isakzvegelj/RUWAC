#!/usr/bin/env python3
"""Actual native import and Secret Service, using generated credentials only."""
import argparse,json,os,subprocess,tempfile,time
from pathlib import Path
from spike import eventually

p=argparse.ArgumentParser()
for name in ['binary','gui','driver','evidence']:p.add_argument('--'+name,type=Path,required=True)
a=p.parse_args()
if os.environ.get('WAYLAND_DISPLAY')!='ruwac-test' or not os.environ.get('RUWAC_TEST_RUN_ROOT'):
 p.error('An owned bounded test display is required')
root=Path(tempfile.mkdtemp(prefix='remmina-passwords-',dir=a.evidence));root.chmod(0o700)
env=dict(os.environ,GTK_A11Y='none',GSETTINGS_BACKEND='memory')
for key,name in [('HOME','home'),('XDG_CONFIG_HOME','config'),('XDG_DATA_HOME','data'),('XDG_CACHE_HOME','cache')]:
 env[key]=str(root/name);Path(env[key]).mkdir(mode=0o700)
folder=Path(env['XDG_DATA_HOME'])/'remmina';folder.mkdir()
def profile(name,marker='.'):
 path=folder/(name+'.remmina')
 path.write_text(f'[remmina]\nname={name}\ngroup=Generated\nprotocol=RDP\nserver=127.0.0.1\nusername=generated\npassword={marker}\n')
 return path
files=[profile('first'),profile('second'),profile('encrypted','GENERATED-CIPHERTEXT')]
before={str(f):f.read_bytes()for f in files}
children=[];seq=0;sock=root/'control.sock';fixture=root/'fixture';fixture.mkdir(mode=0o700);checks=[]
def start(argv,name,stdin=None):
 log=(root/(name+'.log')).open('wb');child=subprocess.Popen(argv,env=env,stdin=subprocess.PIPE if stdin is not None else subprocess.DEVNULL,stdout=log,stderr=log)
 children.append((child,log))
 if stdin is not None:child.stdin.write(stdin);child.stdin.close()
 return child
def secret(args,value=None):
 return subprocess.run(['secret-tool',*args],env=env,input=value,capture_output=True,timeout=10)
def remmina(path,value):
 assert secret(['store','--label=Generated Remmina','xdg:schema','org.remmina.Password','filename',str(path),'key','password'],value).returncode==0
def lookup(reference):
 r=secret(['lookup','application','RUWAC','profile',reference]);return r.stdout.rstrip(b'\n')if r.returncode==0 else None
def command(action,**kw):
 global seq
 seq+=1;tmp=fixture/'request.tmp';tmp.write_text(json.dumps({'seq':seq,'action':action,**kw}));tmp.replace(fixture/'gui-request.json')
 def read():
  try:
   q=json.loads((fixture/'gui-response.json').read_text());return q['result']if q['seq']==seq else None
  except(OSError,ValueError):return None
 return eventually(read,12)
library=Path(env['XDG_CONFIG_HOME'])/'RUWAC/machines.json'
def items():
 try:return json.loads(library.read_text())['items']
 except(OSError,ValueError):return []
def completed():return 'Remmina import complete' in (command('profiles-control').get('status')or'')
try:
 control=root/'keyring';control.mkdir(mode=0o700)
 start(['gnome-keyring-daemon','--foreground','--components=secrets','--control-directory='+str(control),'--unlock'],'keyring',b'generated-disposable-keyring\n')
 time.sleep(.5)
 remmina(files[0],b'generated-remmina-password');remmina(files[1],b'generated-other-password');remmina(files[2],b'generated-stale-keyring-password')
 result=subprocess.run([a.driver,'remmina-credentials',files[0]],env=env,capture_output=True,text=True,timeout=30)
 assert result.returncode==0 and all(json.loads(result.stdout)['result'].values()),result.stdout
 checks.append('actual-Remmina-schema-lookup-copy-and-independent-forget')
 result=subprocess.run([a.driver,'remmina-credentials',files[2]],env=env,capture_output=True,text=True,timeout=15)
 assert result.returncode==0 and json.loads(result.stdout).get('ok') is False, result.stdout
 checks.append('encrypted-file-marker-never-treated-as-plaintext-or-stale-keyring-value')
 start([a.binary,'--socket',sock,'serve'],'daemon');eventually(sock.exists)
 start([a.gui,'--new-instance','--socket',sock,'--fixture-dir',fixture,'--show-machines'],'GUI')
 command('profiles-control',import_expanded=True)
 eventually(lambda:command('profiles-control')['discovered_count']==3,20)
 command('profiles-control',operation='import-all');command('profiles-control',bulk_response='cancel');assert not items()
 checks.append('cancel-leaves-library-and-passwords-unimported')
 command('profiles-control',operation='import-all');command('profiles-control',bulk_response='import')
 eventually(lambda:len(items())==3);eventually(completed,40)
 time.sleep(1.2);assert completed(),'Own-save refresh erased the password-import result'
 first=next(p for p in items()if p['name']=='first');second=next(p for p in items()if p['name']=='second');encrypted=next(p for p in items()if p['name']=='encrypted')
 assert lookup(first['secret_reference'])==b'generated-remmina-password'
 assert lookup(second['secret_reference'])==b'generated-other-password'
 assert lookup(encrypted['secret_reference'])is None
 assert all(not p['preferences'].get('gnome_secret_reference')for p in items())
 assert all(b'generated-remmina-password'not in path.read_bytes()for path in [library,library.with_name('machines.sqlite3')])
 checks.append('native-bulk-import-exact-profile-passwords-without-GNOME-or-metadata-secrets')
 remmina(files[0],b'generated-changed-source-password')
 command('profiles-control',operation='import-all');command('profiles-control',bulk_response='import');eventually(completed,40)
 assert lookup(first['secret_reference'])==b'generated-remmina-password'
 checks.append('existing-RUWAC-password-preserved-on-duplicate-import')
 extra=fixture/'optout.remmina';extra.write_bytes(profile('optout').read_bytes());remmina(extra,b'generated-optout-password')
 command('profiles-control',file=extra.name,import_passwords=False,operation='import')
 eventually(lambda:len(items())==4)
 target=next(p for p in items()if p['name']=='optout');assert lookup(target['secret_reference'])is None
 checks.append('native-password-checkbox-opt-out-keeps-metadata-only')
 command('profiles-control',file=extra.name,import_passwords=True,operation='import');eventually(completed,40)
 assert len(items())==4 and lookup(target['secret_reference'])==b'generated-optout-password'
 checks.append('reimport-repairs-missing-password-without-duplicate-identity')
 assert all(path.read_bytes()==before[str(path)]for path in files)
 checks.append('source-Remmina-files-unchanged')
 (root/'acceptance.json').write_text(json.dumps({'checks':checks,'generated_secrets_only':True},indent=2)+'\n')
 print('Remmina native password import PASS',len(checks),'groups',flush=True)
finally:
 for child,log in reversed(children):
  if child.poll()is None:child.terminate()
  try:child.wait(timeout=8)
  except subprocess.TimeoutExpired:child.kill();child.wait(timeout=3)
  log.close()
