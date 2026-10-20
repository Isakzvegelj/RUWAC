#!/usr/bin/env python3
"""Actual owned GTK RPM controls with generated administrator/helper responses."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import time
from spike import Client, eventually


def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--binary',type=Path,required=True);p.add_argument('--gui',type=Path,required=True)
    p.add_argument('--evidence',type=Path,required=True);a=p.parse_args()
    if not os.environ.get('WAYLAND_DISPLAY','').startswith('ruwac-'):
        p.error('An explicitly owned test Wayland display is required')
    a.evidence.mkdir(parents=True,exist_ok=True)
    root=Path(tempfile.mkdtemp(prefix='native-rpm-updates-',dir=a.evidence));root.chmod(0o700)
    env={**os.environ,'GTK_A11Y':'none','GSETTINGS_BACKEND':'memory'}
    for key,name in [('XDG_CONFIG_HOME','config'),('XDG_DATA_HOME','data'),('XDG_CACHE_HOME','cache')]:
        env[key]=str(root/name);Path(env[key]).mkdir(mode=0o700)
    prefix=root/'usr';(prefix/'bin').mkdir(parents=True);(prefix/'share/ruwac').mkdir(parents=True)
    for source,name in [(a.binary,'ruwac'),(a.gui,'ruwac-desktop')]:shutil.copy2(source,prefix/'bin'/name)
    (prefix/'share/ruwac/rpm-installation.json').write_text('{"kind":"fedora-testing-rpm"}\n')
    helpers=root/'helpers';helpers.mkdir();stub=helpers/'python3'
    stub.write_text('''#!/usr/bin/python3
import json,os,sys,time
from pathlib import Path
r=Path(os.environ['RUWAC_RPM_UI_FIXTURE']);args=sys.argv[3:]
with (r/'calls.jsonl').open('a')as f:f.write(json.dumps(args)+'\\n')
assert '--managed-rpm'in args
if '--auto-install'in args:
 mode=args[-1];file=r/'enabled'
 if mode!='status':file.write_text(mode)
 if mode=='off'and(r/'fail-stop').exists():
  print(json.dumps({'error':'Generated timer-stop failure after saved opt-out'}));sys.exit(1)
 print(json.dumps({'distribution':'rpm','automatic_install':not file.exists()or file.read_text()=='on'}))
elif '--install'in args:
 time.sleep(1.5);print(json.dumps({'distribution':'rpm','installed':'9.9.9'}))
else:print(json.dumps({'distribution':'rpm','available':True,'latest':'9.9.9',
 'latest_published':'9.9.10','latest_verified':'9.9.9','installed_version':'9.9.8',
 'running':args[args.index('--current')+1], 'running_executable':args[args.index('--running-executable')+1],
 'mixed_installation':True,'system_installed':'9.9.8','user_installed':'9.9.7',
 'skipped_releases':[{'tag':'v9.9.10','reason':'Exact-commit quality: failure'}]}))
''');stub.chmod(0o700)
    env['PATH']=str(helpers)+':'+env['PATH'];env['RUWAC_RPM_UI_FIXTURE']=str(root)
    children=[];seq=0;results=[]
    def start(args,name):
        f=(root/(name+'.log')).open('w');child=subprocess.Popen(args,env=env,stdout=f,stderr=f);children.append((child,f));return child
    def command(action,**kw):
        nonlocal seq
        seq+=1;tmp=root/'request.tmp';tmp.write_text(json.dumps({'seq':seq,'action':action,**kw}));tmp.replace(root/'gui-request.json')
        def read():
            try:value=json.loads((root/'gui-response.json').read_text());return value['result']if value['seq']==seq else None
            except(OSError,ValueError):return None
        return eventually(read,10)
    try:
        sock=root/'control.sock';daemon=start([str(prefix/'bin/ruwac'),'--socket',str(sock),'serve'],'daemon');eventually(lambda:sock.exists())
        with Client(sock)as c:s=c.call('session.create',{'name':'Generated RPM UI preservation','argv':['/bin/cat']})
        start([str(prefix/'bin/ruwac-desktop'),'--socket',str(sock),'--session',s['session'],'--fixture-dir',str(root)],'gui')
        eventually(lambda:command('inspect'),15)
        command('updates',operation='open')
        eventually(lambda:command('updates')['automatic_install_enabled'])
        eventually(lambda:command('updates')['install_enabled'])
        assert command('updates')['automatic_install']is True
        text=command('updates')['status']
        for label in ['Latest published tag: 9.9.10','Latest verified installable: 9.9.9','Installed version: 9.9.8','Running frontend:','Both installations detected:','Skipped v9.9.10: Exact-commit quality: failure']:
            assert label in text,(label,text)
        assert str(prefix/'bin/ruwac-desktop') in text
        results.append('four-version-states-mixed-installation-running-path-and-precise-skip-reason')
        time.sleep(.3)
        calls=[json.loads(line)for line in (root/'calls.jsonl').read_text().splitlines()]
        assert not any(args[-2:]in [['--auto-install','on'],['--auto-install','off']]for args in calls),'Status initialization requested administrator authorization'
        results.append('initial-enabled-state-never-reconfigures-or-authorizes')
        (root/'fail-stop').touch()
        command('updates',automatic_install=False)
        eventually(lambda:(root/'enabled').exists()and(root/'enabled').read_text()=='off')
        eventually(lambda:command('updates')['automatic_install_enabled'])
        assert command('updates')['automatic_install']is False
        results.append('failed-timer-stop-refreshes-saved-opt-out')
        command('updates',automatic_install=True)
        eventually(lambda:(root/'enabled').read_text()=='on')
        eventually(lambda:command('updates')['automatic_install_enabled'])
        assert command('updates')['automatic_install']is True
        results.append('native-rpm-opt-in-and-opt-out')
        before=command('inspect')['ticks'];response=command('updates',click='update-install');assert not response['install_enabled']
        time.sleep(.3);assert command('inspect')['ticks']>before+8
        with Client(sock)as c:
            c.call('pane.sendText',{'pane':s['pane'],'text':'RPM_UI_REAL_PTY_PRESERVED\n'})
            eventually(lambda:'RPM_UI_REAL_PTY_PRESERVED'in c.call('pane.capture',{'pane':s['pane']})['text'])
        eventually(lambda:'Installed RPM'in command('updates')['status'])
        assert daemon.poll()is None
        results.append('native-install-worker-keeps-gtk-and-real-pty-responsive')
        command('updates',operation='close');assert not command('updates')['dialog']
        calls=[json.loads(line)for line in (root/'calls.jsonl').read_text().splitlines()]
        assert sum(args[-2:]==['--auto-install','on']for args in calls)==1
        assert sum(args[-2:]==['--auto-install','off']for args in calls)==1
        assert sum(args[-1:]==['--install']for args in calls)==1
        assert not (Path(env['XDG_CONFIG_HOME'])/'RUWAC/updates.json').exists()
        results.append('no-duplicate-operations-or-per-user-installation-shadow')
        receipt={'results':results,'hashes':{name:hashlib.sha256((prefix/'bin'/name).read_bytes()).hexdigest()for name in ['ruwac','ruwac-desktop']},'scope':'Actual compiled GTK on owned Wayland and real PTY; generated helper/admin responses and synthetic callbacks, not real polkit authorization, physical input or DNF installation'}
        (root/'acceptance.json').write_text(json.dumps(receipt,indent=2)+'\n')
        print('Native RPM update controls PASS',len(results),'groups; evidence',root)
    finally:
        for child,f in reversed(children):
            if child.poll()is None:child.terminate()
            try:child.wait(timeout=10)
            except subprocess.TimeoutExpired:child.kill();child.wait()
            f.close()


if __name__=='__main__':main()
