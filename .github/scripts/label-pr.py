#!/usr/bin/env python3
"""Base-branch-only, add-only contribution labels. Never execute PR code."""
import argparse,json,os,re,urllib.request
from pathlib import Path
LABELS={'area/terminal':('268bd2','Persistent engine, CLI and terminal input'),'area/ui':('6f42c1','Native desktop workspace'),'area/rdp':('0075ca','Embedded RDP transport/display'),'area/clipboard':('0e8a16','Clipboard ownership or transfer'),'area/files':('d4c5f9','SSH/SFTP and file workspaces'),'area/docs':('5319e7','Documentation or governance'),'area/ci':('fbca04','Repository checks and workflows')}
def classify(paths):
 result=set()
 for raw in paths:
  if not isinstance(raw,str) or raw.startswith('/') or '..' in Path(raw).parts:raise ValueError('Invalid changed-file path')
  p=raw.lower()
  if p.startswith('ruwac/crates/') and 'ruwac-clipboard/' not in p:result.add('area/terminal')
  if p.startswith(('ruwac/desktop/','ruwac/gui/')):result.add('area/ui')
  if 'rdp' in p:result.add('area/rdp')
  if 'clipboard' in p or p.endswith('/clip.rs'):result.add('area/clipboard')
  if 'sftp' in p or '/files' in p:result.add('area/files')
  if p.endswith('.md') or p.startswith(('spec/','architecture/')):result.add('area/docs')
  if p.startswith('.github/') or p.startswith('scripts/'):result.add('area/ci')
 return sorted(result)
def api(method,path,body=None):
 req=urllib.request.Request('https://api.github.com/repos/w3fans/RUWAC'+path,data=None if body is None else json.dumps(body).encode(),method=method,headers={'Authorization':'Bearer '+os.environ['GITHUB_TOKEN'],'Accept':'application/vnd.github+json','X-GitHub-Api-Version':'2022-11-28','Content-Type':'application/json'})
 with urllib.request.urlopen(req,timeout=20)as r:
  data=r.read(4*1024*1024+1)
  if len(data)>4*1024*1024:raise ValueError('API response limit')
  return json.loads(data)
def main():
 p=argparse.ArgumentParser();p.add_argument('--files',type=Path);a=p.parse_args()
 if a.files:print(json.dumps({'labels':classify(json.loads(a.files.read_text()))}));return
 event=json.loads(Path(os.environ['GITHUB_EVENT_PATH']).read_text());pr=event['pull_request'];number=event['number']
 if os.environ.get('GITHUB_REPOSITORY')!='w3fans/RUWAC' or event['repository']['full_name']!='w3fans/RUWAC' or type(number)is not int or not 1<=number<=2147483647:raise ValueError('Repository/event target rejected')
 if pr['base']['repo']['full_name']!='w3fans/RUWAC':raise ValueError('PR base rejected')
 # Verify checkout is exactly the trusted base commit before using write authority.
 import subprocess
 if subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip()!=pr['base']['sha']:raise ValueError('Expected trusted base checkout')
 paths=[]
 for page in range(1,32):
  rows=api('GET',f'/pulls/{number}/files?per_page=100&page={page}');paths.extend(row['filename']for row in rows)
  if len(rows)<100:break
 else:raise ValueError('Changed-file pagination limit')
 if len(paths)!=pr['changed_files']:raise ValueError('Changed-file set incomplete; labels left unchanged')
 desired=classify(paths)
 if not desired:return
 known=set()
 for page in range(1,11):
  rows=api('GET',f'/labels?per_page=100&page={page}');known.update(row['name']for row in rows)
  if len(rows)<100:break
 else:raise ValueError('Repository label pagination limit')
 for label in desired:
  if label not in known:
   color,description=LABELS[label];api('POST','/labels',{'name':label,'color':color,'description':description})
 api('POST',f'/issues/{number}/labels',{'labels':desired});print(json.dumps({'applied_labels':desired}))
if __name__=='__main__':main()
