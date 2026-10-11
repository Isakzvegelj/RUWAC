#!/usr/bin/env python3
"""Check locked declarations and the independent headless dependency graph."""
import argparse,hashlib,json,subprocess,tomllib
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
BANNED=('gtk','gdk','glib','gio','libadwaita','ironrdp','ssh2','cairo','pango')
def inspect(name,manifest):
 lock=tomllib.loads(manifest.with_name('Cargo.lock').read_text());locked={(p['name'],p['version'],p.get('source')):p for p in lock['package']}
 metadata=json.loads(subprocess.check_output(['cargo','metadata','--locked','--format-version','1','--manifest-path',str(manifest)],text=True))
 rows=[]
 for p in metadata['packages']:
  source=p.get('source');key=(p['name'],p['version'],source);entry=locked.get(key)
  if entry is None:raise ValueError('Resolved package absent from lock: '+p['name'])
  license_text=p.get('license');license_file=p.get('license_file');file_hash=None
  if not license_text and not license_file:raise ValueError('Missing declared license: '+p['name'])
  if license_file:
   package_root=Path(p['manifest_path']).parent.resolve();path=(package_root/license_file).resolve()
   if not path.is_relative_to(package_root) or path.stat().st_size>2*1024*1024:raise ValueError('Declared license file outside package or oversized: '+p['name'])
   if not path.is_file():raise ValueError('Declared license file missing: '+p['name'])
   file_hash=hashlib.sha256(path.read_bytes()).hexdigest()
  if source and source.startswith('registry+') and not entry.get('checksum'):raise ValueError('Registry checksum missing: '+p['name'])
  if name=='headless' and p['name'].startswith(BANNED):raise ValueError('Graphical/embedded remote dependency in headless graph: '+p['name'])
  rows.append({'name':p['name'],'version':p['version'],'declared_license':license_text,'license_file_sha256':file_hash,'registry_checksum':entry.get('checksum')})
 return {'packages':sorted(rows,key=lambda p:(p['name'],p['version'])),'package_count':len(rows),'lock_sha256':hashlib.sha256(manifest.with_name('Cargo.lock').read_bytes()).hexdigest()}
def main():
 p=argparse.ArgumentParser();p.add_argument('--output',type=Path,required=True);a=p.parse_args()
 result={'headless':inspect('headless',ROOT/'ruwac/Cargo.toml'),'desktop':inspect('desktop',ROOT/'ruwac/desktop/Cargo.toml'),'scope':'Locked declaration/checksum and graphical-dependency exclusion checks; not legal approval, binary notice clearance or a vulnerability scan'}
 a.output.parent.mkdir(parents=True,exist_ok=True);a.output.write_text(json.dumps(result,indent=2)+'\n');print('Dependency declarations and headless graph PASS',flush=True)
if __name__=='__main__':main()
