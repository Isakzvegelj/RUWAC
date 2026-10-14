#!/usr/bin/env python3
"""Actual provenance/DNF update engine in a disposable Fedora root; cached API replay."""
import argparse
import importlib.util
import json
from pathlib import Path
import re
import shutil
import sys

ROOT = Path(__file__).resolve().parents[3]
DRIVER = '''
import hashlib,importlib.util,json,os,shutil,subprocess
from pathlib import Path
root=Path('/packages/.updater-fixture')
def module(name,path):
 spec=importlib.util.spec_from_file_location(name,path);value=importlib.util.module_from_spec(spec);spec.loader.exec_module(value);return value
rpm=module('rpm_updates',root/'rpm-update.py');helper=module('release_updates',root/'update-user.py')
data=json.loads((root/'release.json').read_text());commit=(root/'commit').read_text().strip();tag=data['tag_name'];latest=tag[1:]
release={'latest':latest,'tag':tag,'commit':commit}
helper.latest=lambda current:dict(release,current=current,available=helper.version(latest)>helper.version(current))
# Metadata and quality responses are replayed; official cryptographic verification,
# RPM queries, DNF transactions, installed versions and persistent PTYs are real.
helper.automatic_release_verified=lambda value:True
def api(path):
 assert path=='/releases/tags/'+tag
 return data
helper.api=api
def download(asset,destination):
 source=root/'assets'/asset['name'];payload=source.read_bytes()
 assert len(payload)==asset['size'] and 'sha256:'+hashlib.sha256(payload).hexdigest()==asset['digest']
 destination.write_bytes(payload)
helper.download_bundle=download
engine=rpm.Updater(helper,Path('/tmp/ruwac-rpm-test/updater-state'),Path('/tmp/ruwac-rpm-test/updater-config/rpm-updates.json'))
initial,components=engine.installed();assert helper.version(initial)<helper.version(latest)
rpm.save_policy(True,engine.config)
release['commit']='b'*40
try:engine.install(True)
except ValueError:pass
else:raise AssertionError('Wrong source attestation accepted')
assert engine.installed()==(initial,components)
release['commit']=commit
def optout(asset,destination):download(asset,destination);rpm.save_policy(False,engine.config)
helper.download_bundle=optout
assert 'skipped'in engine.install(True)
assert engine.installed()==(initial,components)
rpm.save_policy(True,engine.config);helper.download_bundle=download
result=engine.install(True);assert result['installed']==latest
assert engine.installed()==(latest,components)
subprocess.run(['/usr/bin/dnf','-y','upgrade','--no-allow-downgrade',*map(str,Path('/previous').glob('*.rpm'))],check=True)
assert engine.installed()==(latest,components),'Stale local package downgraded installed release'
session=json.loads(Path('/tmp/ruwac-rpm-test/session.json').read_text());os.kill(session['pid'],0)
assert not Path('/etc/ruwac/rpm-updates.json').exists(),'Fixture must not opt in the actual installed application'
print('RPM_UPDATER_REAL_DNF_PASS '+json.dumps({'tag':tag,'commit':commit,'wrong_source_refused':True,'mid_download_optout_preserved':True,'official_RPM_attestations_verified':True,'actual_DNF_upgrade':True,'stale_local_RPM_upgrade_did_not_downgrade':True,'same_PTY_child_survived':True,'scope':'Disposable Fedora root, real official verifier/RPM/DNF; cached asset/API/quality replay, not physical desktop or unattended system timer'}),flush=True)
'''


def main():
    p=argparse.ArgumentParser(description=__doc__)
    for name in ['rpms','previous-rpms','evidence','signed-rpms','release-metadata']:
        p.add_argument('--'+name,type=Path,required=True)
    p.add_argument('--release-commit',required=True)
    p.add_argument('--podman-root',type=Path)
    p.add_argument('--podman-runroot',type=Path)
    p.add_argument('--run-owned-fixture',action='store_true')
    a=p.parse_args()
    if not a.run_owned_fixture or not re.fullmatch('[0-9a-f]{40}',a.release_commit):
        p.error('Explicit owned fixture and immutable source commit are required')
    data=json.loads(a.release_metadata.read_text());tag=data['tag_name']
    if not re.fullmatch(r'v[0-9]+\.[0-9]+\.[0-9]+',tag):p.error('Invalid release tag')
    stage=a.rpms/'.updater-fixture'
    stage.mkdir(mode=0o700)
    (stage/'assets').mkdir(mode=0o700)
    for component in ['ruwac','ruwac-desktop']:
        name=component+'-'+tag[1:]+'-1.fc44.x86_64.rpm'
        for suffix in ['', '.sigstore.json']:
            shutil.copyfile(a.signed_rpms/(name+suffix),stage/'assets'/(name+suffix))
    for name in ['rpm-update.py','update-user.py']:
        shutil.copyfile(ROOT/'scripts'/name,stage/name)
    shutil.copyfile(a.release_metadata,stage/'release.json');(stage/'commit').write_text(a.release_commit+'\n')
    spec=importlib.util.spec_from_file_location('lifecycle',Path(__file__).with_name('rpm_dnf_lifecycle.py'))
    lifecycle=importlib.util.module_from_spec(spec);spec.loader.exec_module(lifecycle)
    marker='dnf -y install /packages/*.rpm\n'
    assert lifecycle.BODY.count(marker)==1
    phase="dnf -y install gh\npython3 - <<'UPDATER'\n"+DRIVER+"\nUPDATER\ndnf -y downgrade /previous/*.rpm\n"
    lifecycle.BODY=lifecycle.BODY.replace(marker,phase+marker)
    before=set(a.evidence.glob('fedora-dnf-*'))
    sys.argv=[sys.argv[0],'--rpms',str(a.rpms),'--previous-rpms',str(a.previous_rpms),'--evidence',str(a.evidence),'--run-owned-fixture']
    for flag,value in [('--podman-root',a.podman_root),('--podman-runroot',a.podman_runroot)]:
        if value:sys.argv.extend([flag,str(value)])
    lifecycle.main()
    created=set(a.evidence.glob('fedora-dnf-*'))-before;assert len(created)==1
    evidence=created.pop();lines=(evidence/'dnf.log').read_text().splitlines()
    results=[json.loads(line.split('RPM_UPDATER_REAL_DNF_PASS ',1)[1])for line in lines if line.startswith('RPM_UPDATER_REAL_DNF_PASS ')]
    assert len(results)==1
    (evidence/'rpm-updater-acceptance.json').write_text(json.dumps(results[0],indent=2)+'\n')
    print('Actual signed RPM verification, opt-out, DNF update and live-child persistence PASS')


if __name__=='__main__':main()
