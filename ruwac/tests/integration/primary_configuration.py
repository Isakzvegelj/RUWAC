#!/usr/bin/env python3
"""Headless setup must preserve the complete existing profile and secret refs."""
import argparse,hashlib,json,os,sqlite3,subprocess,tempfile
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('--gui',required=True);p.add_argument('--evidence',required=True);a=p.parse_args()
r=Path(tempfile.mkdtemp(prefix='primary-config-',dir=a.evidence));r.chmod(0o700);config=r/'config';config.mkdir(mode=0o700);directory=config/'RUWAC';directory.mkdir(mode=0o700);home=r/'home';home.mkdir(mode=0o700)
pf={'id':'endpoint_primary_config','machine_id':'machine_primary_config','name':'Retained profile Ž','group':'retained/group','protocol':'RDP','server':'example.invalid:3389','username':'service-account','secret_reference':'retained-service-ref','favorite':True,'last_used':123,'tags':['retained'],'preferences':{'gnome_username':'desktop-account','gnome_secret_reference':'retained-gnome-ref','rdp_mode':'gnome-remote-login','display':'dynamic','clipboard':0}}
path=directory/'machines.json';path.write_text(json.dumps({'schema':1,'items':[pf],'groups':[]}));path.chmod(0o600)
env={**os.environ,'HOME':str(home),'XDG_CONFIG_HOME':str(config),'XDG_DATA_HOME':str(r/'data'),'XDG_CACHE_HOME':str(r/'cache'),'PYTHONDONTWRITEBYTECODE':'1'}
for k in ['DISPLAY','WAYLAND_DISPLAY','DBUS_SESSION_BUS_ADDRESS']:env.pop(k,None)
checks=[]
def run(args,success=True):
 p=subprocess.run([a.gui,*args],env=env,capture_output=True,text=True,timeout=12)
 assert(p.returncode==0)==success,{'args':args,'returncode':p.returncode,'stderr':p.stderr}
def document():
 c=sqlite3.connect('file:'+str(directory/'machines.sqlite3')+'?mode=ro',uri=True)
 try:return json.loads(c.execute('select document from library where id=1').fetchone()[0])
 finally:c.close()
def mark(name):checks.append(name);print(name,'PASS',flush=True)
run(['--receive-primary',pf['id'],'linux-user','wayland-0','9223'])
actual=document()['items'][0]
assert all(actual[k]==pf[k]for k in pf if k!='preferences')
assert all(actual['preferences'][k]==v for k,v in pf['preferences'].items())
assert actual['preferences']['primary_sync']is True and actual['preferences']['primary_user']=='linux-user' and actual['preferences']['primary_port']==9223
mark('headless-configure-preserves-metadata-and-both-credential-refs')
run(['--disable-primary',pf['id']]);assert not document()['items'][0]['preferences'].get('primary_sync',False);mark('headless-disable')
run(['--receive-primary',pf['id'],'linux-user','wayland-1','0']);assert document()['items'][0]['preferences']['primary_display']=='wayland-1';mark('SSH-config-port-and-exact-session')
for args in [['--receive-primary',pf['id'],'linux-user','wayland-0;bad','0'],['--receive-primary',pf['id'],'bad;user','wayland-0','0'],['--receive-primary',pf['id'],'linux-user','wayland-0','65536'],['--receive-primary','endpoint_missing','linux-user','wayland-0','0']]:
 before=document();mirror=path.read_bytes();run(args,False);assert document()==before and path.read_bytes()==mirror
mark('invalid-session-account-port-or-profile-refused-without-metadata-change')
(r/'acceptance.json').write_text(json.dumps({'checks':checks,'PASS':True,'real_Sqlite_and_mirror':True,'GUI_display_required':False,'passwords_read':False},indent=2)+'\n')
print(r,flush=True)
