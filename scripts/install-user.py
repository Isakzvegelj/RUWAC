#!/usr/bin/env python3
"""Reversible per-user installation of built RUWAC artifacts; never allocates a version."""
import argparse, fcntl, hashlib, json, os, re, shutil, stat, subprocess, tempfile, tomllib
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('--build-root',type=Path,required=True);p.add_argument('--desktop',action='store_true');p.add_argument('--prefix',type=Path);a=p.parse_args()
root=a.build_root.resolve();prefix=(a.prefix or Path.home()/'.local').absolute()
version=tomllib.loads((root/'ruwac/Cargo.toml').read_text())['workspace']['package']['version']
if not re.fullmatch(r'(0|[1-9][0-9]*)\.[0-9]\.([0-9]|[1-9][0-9])',version):raise SystemExit('Invalid controller-assigned numeric version')
if any(c=='='or ord(c)<32 for c in str(prefix)):raise SystemExit('Unsupported desktop executable path')
files=['ruwac']+(['ruwac-desktop'] if a.desktop else [])
artifacts={name:root/'target/release'/name for name in files}
for name,path in artifacts.items():
 if not path.is_file()or path.is_symlink():raise SystemExit(f'Expected built artifact: {name}')
 output=subprocess.check_output([str(path),'--version'],text=True).strip()
 if not output.endswith(' '+version):raise SystemExit('Artifact version does not match source')
base=prefix/'lib/ruwac';base.mkdir(parents=True,exist_ok=True)
if base.is_symlink() or base.stat().st_uid!=os.geteuid():raise SystemExit('Unsafe installation directory')
lock=os.open(base/'.install.lock',os.O_CREAT|os.O_RDWR|os.O_NOFOLLOW,0o600);m=os.fstat(lock)
if not stat.S_ISREG(m.st_mode)or m.st_uid!=os.geteuid()or m.st_nlink!=1 or m.st_mode&0o077:raise SystemExit('Unsafe installation lock')
try:fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
except BlockingIOError:raise SystemExit('Another RUWAC installation is active')
release=base/'releases'/version;release.parent.mkdir(exist_ok=True)
if release.parent.is_symlink():raise SystemExit('Unsafe releases directory')
manifest={'version':version,'artifacts':{name:hashlib.sha256(path.read_bytes()).hexdigest()for name,path in artifacts.items()},'source_commit':subprocess.check_output(['git','-C',str(root),'rev-parse','HEAD'],text=True).strip(),'dirty':bool(subprocess.check_output(['git','-C',str(root),'diff','--name-only'],text=True).strip())}
receipt=release/'installation.json'
if release.exists():
 if release.is_symlink() or not receipt.is_file() or json.loads(receipt.read_text())!=manifest:raise SystemExit('Existing immutable installation differs; preserve it and use a new controller-assigned release')
 for name,hash in manifest['artifacts'].items():
  if hashlib.sha256((release/name).read_bytes()).hexdigest()!=hash:raise SystemExit('Existing installation artifact has changed')
else:
 temp=Path(tempfile.mkdtemp(prefix='.install-',dir=release.parent))
 try:
  for name,path in artifacts.items():shutil.copyfile(path,temp/name);(temp/name).chmod(0o755)
  (temp/'installation.json').write_text(json.dumps(manifest,indent=2)+'\n');(temp/'installation.json').chmod(0o600)
  temp.rename(release)
 finally:
  if temp.exists():shutil.rmtree(temp)
def link(path,target,allowed):
 if path.exists() or path.is_symlink():
  if not path.is_symlink() or str(path.readlink())not in allowed:raise SystemExit(f'Refusing unrelated installed path: {path}')
 tmp=path.with_name(path.name+'.ruwac-new');
 if tmp.exists()or tmp.is_symlink():raise SystemExit(f'Installation lock conflict: {tmp}')
 tmp.symlink_to(target);tmp.replace(path)
current=base/'current';old=str(current.readlink())if current.is_symlink()else None
if current.exists()and not current.is_symlink():raise SystemExit('Refusing unrelated current path')
if old and (not old.startswith('releases/')or '..'in Path(old).parts):raise SystemExit('Unsafe current link')
bin=prefix/'bin';bin.mkdir(parents=True,exist_ok=True)
if bin.is_symlink()or bin.stat().st_uid!=os.geteuid():raise SystemExit('Unsafe command directory')
if not a.desktop and (bin/'ruwac-desktop').is_symlink():raise SystemExit('Retain the desktop installation with --desktop')
# Check all externally visible paths before changing current.
for name in files:
 path=bin/name;target=str(Path('..')/'lib/ruwac/current'/name)
 if (path.exists()or path.is_symlink())and (not path.is_symlink()or str(path.readlink())!=target):raise SystemExit(f'Command collision: {name}')
apps=prefix/'share/applications';launcher=apps/'si.z4.ruwac.desktop'
# Desktop Entry string escaping precedes Exec argument unquoting.
encoded=''.join(('\\'*4 if c=='\\' else '\\'*2+c if c in '"$`' else '%%' if c=='%' else c)for c in str(bin/'ruwac'))
text='[Desktop Entry]\nType=Application\nName=RUWAC\nComment=Persistent terminals, remote desktops and files\nExec="'+encoded+'" gui\nIcon=utilities-terminal\nTerminal=false\nCategories=System;TerminalEmulator;RemoteAccess;\nStartupNotify=true\n'

if a.desktop and apps.is_symlink():raise SystemExit('Unsafe launcher directory')
if a.desktop and launcher.exists()and launcher.read_text()!=text:raise SystemExit('Refusing unrelated desktop launcher')
previous=base/'previous'
if previous.is_symlink()and (not str(previous.readlink()).startswith('releases/')or '..'in previous.readlink().parts):raise SystemExit('Unsafe previous link')
if old and old!='releases/'+version:link(base/'previous',old,{str((base/'previous').readlink())}if(base/'previous').is_symlink()else set())
link(current,'releases/'+version,{old}if old else set())
for name in files:link(bin/name,str(Path('..')/'lib/ruwac/current'/name),{str(Path('..')/'lib/ruwac/current'/name)})
if a.desktop:
 apps.mkdir(parents=True,exist_ok=True)
 if apps.is_symlink()or apps.stat().st_uid!=os.geteuid():raise SystemExit('Unsafe launcher directory')
 launcher.write_text(text);launcher.chmod(0o644)
 if shutil.which('update-desktop-database'):subprocess.run(['update-desktop-database',str(apps)],check=True)
print(json.dumps({'installed':version,'command':str(bin/'ruwac'),'launcher':str(launcher)if a.desktop else None,'receipt':str(receipt),'previous':old},indent=2))
