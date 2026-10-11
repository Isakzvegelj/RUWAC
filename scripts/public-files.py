#!/usr/bin/env python3
"""Explicit repository inventory. Private and generated material is never inferred."""
from pathlib import Path
import argparse,ast,json,sys
ROOT_FILES={'.gitignore','README.md','CHANGELOG.md','RELEASE-NOTES.md','LICENSE'}
ARCH_FILES={f'{n:02}-{name}.md' for n,name in enumerate(['RUWAC-DECISION','ARCHITECTURE','PROTOCOL-V0','ACCEPTANCE-MATRIX','BENCHMARKS','OPEN-ISSUES','IMPLEMENTATION-ROADMAP','COMPONENTS-LICENSES'])}|{'evidence-index.json','dependency-licenses.json','benchmark-results.json','acceptance-results.json'}
SCRIPT_FILES={'public-files.py','install-user.py'}
FORBIDDEN={'.agents','.codex','.claude','.kimi','.githooks','.vscode','.aws','AGENTS.md','CLAUDE.md','SKILL.md','FIRST-PROMPT.md','OWNER-SPIKE-SPEC.md'}
def files(root):
 out=[]
 candidates=[root/f for f in ROOT_FILES]
 for directory in ['architecture','scripts','ruwac','.github','spec']:
  candidates.extend((root/directory).rglob('*'))
 for p in candidates:
  r=p.relative_to(root)
  if p.is_symlink():continue
  if not p.is_file():continue
  if any(x in FORBIDDEN or x in {'target','.git','__pycache__'} for x in r.parts):continue
  if len(r.parts)==1 and r.name in ROOT_FILES:out.append(r)
  elif r.parts[0]=='spec' and len(r.parts)==2 and r.suffix=='.md':out.append(r)
  elif r.parts[0]=='ruwac' and r.parts[1]=='desktop' and (r.name in {'Cargo.toml','Cargo.lock'} or r.suffix=='.rs'):out.append(r)
  elif r.parts[0]=='ruwac' and r.parts[1]=='gui' and r.suffix=='.py':out.append(r)
  elif r.parts[0]=='architecture' and len(r.parts)==2 and r.name in ARCH_FILES:out.append(r)
  elif r.as_posix()=='.github/workflows/release.yml':out.append(r)
  elif r.parts[0]=='scripts' and len(r.parts)==2 and r.name in SCRIPT_FILES:out.append(r)
  elif r.parts[0]=='ruwac' and ((r.name in {'Cargo.toml','Cargo.lock'} and len(r.parts)<=4) or (r.suffix=='.rs' and 'src' in r.parts) or (r.suffix=='.py' and r.parts[1]=='tests')):out.append(r)
 return sorted(out)
def verify(root,paths):
 allowed=set(files(root));bad=[]
 for name in paths:
  p=Path(name)
  if p not in allowed:bad.append(f'not allowlisted: {p}');continue
  data=(root/p).read_bytes()
  for marker in [b'/app/',b'/zDev/',b'/oclaw/']:
   if p.as_posix()!='scripts/public-files.py' and marker in data:bad.append(f'machine-specific path: {p}')
  if p.as_posix()=='scripts/public-files.py':continue
  for marker in [b'BEGIN OPENSSH PRIVATE KEY',b'BEGIN RSA PRIVATE KEY']:
   if marker in data:bad.append(f'private content marker: {p}')
 if bad:raise SystemExit('\n'.join(bad))
 return len(paths)
if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('--root',type=Path,default=Path(__file__).resolve().parents[1]);p.add_argument('--check',action='store_true');p.add_argument('--tracked',action='store_true');a=p.parse_args();root=a.root.resolve()
 if a.tracked:
  import subprocess
  tracked=subprocess.check_output(['git','-C',str(root),'ls-files','-z']).decode().split('\0');tracked=[x for x in tracked if x];print('Verified repository inventory:',verify(root,tracked),'files')
 elif a.check:
  inventory=files(root);verify(root,[str(x) for x in inventory])
  for r in inventory:
   if r.suffix=='.py':ast.parse((root/r).read_text(),filename=str(r))
  if (root/'ruwac/target').exists():raise SystemExit('build artifacts in source workspace')
  print('Source inventory checked:',len(inventory),'files')
 else:
  for r in files(root):print(r.as_posix())
