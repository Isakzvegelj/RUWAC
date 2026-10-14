#!/usr/bin/env python3
"""Explicit disposable Fedora RAM-root DNF lifecycle; never modifies host RPM state."""
import argparse
import json
import os
from pathlib import Path
import subprocess
import tempfile
import uuid

IMAGE = 'registry.fedoraproject.org/fedora@sha256:05e3c351a7e762aca8fb067a9a92c1f775a7c4fbb1382c97cdd87bd6fb5b162c'
BODY = 'set -euo pipefail\nmkdir -m 700 /tmp/ruwac-rpm-test\nprintf \'%s\\n\' \'{"generated":"private configuration sentinel"}\' > /tmp/ruwac-rpm-test/profile.json\nsha256sum /tmp/ruwac-rpm-test/profile.json > /tmp/ruwac-rpm-test/profile.sha256\ndnf -y install /previous/*.rpm\n/usr/bin/ruwac --version\n/usr/bin/ruwac-desktop --version\n/usr/bin/ruwac --socket /tmp/ruwac-rpm-test/control.sock serve > /tmp/ruwac-rpm-test/daemon.log 2>&1 &\ndaemon=$!\ntrap \'kill "$daemon" 2>/dev/null || true; wait "$daemon" 2>/dev/null || true\' EXIT\npython3 - <<\'CREATE\'\nimport json,subprocess,time\nfrom pathlib import Path\nbinary=\'/usr/bin/ruwac\';base=[binary,\'--socket\',\'/tmp/ruwac-rpm-test/control.sock\']\nfor _ in range(100):\n p=subprocess.run(base+[\'health\'],capture_output=True,text=True)\n if p.returncode==0:break\n time.sleep(.05)\nelse:raise SystemExit(\'Daemon readiness failed\')\nreceiver=Path(\'/tmp/ruwac-rpm-test/receiver.py\')\nreceiver.write_text("import os,tty\\nfrom pathlib import Path\\ntty.setraw(0)\\nprint(\'GENERATED_READY\',flush=True)\\np=Path(\'/tmp/ruwac-rpm-test/input.bin\')\\np.touch()\\nwhile True:\\n b=os.read(0,65536)\\n if not b:break\\n with p.open(\'ab\')as f:f.write(b)\\n")\nreply=json.loads(subprocess.check_output(base+[\'rpc\',\'session.create\',\'--params\',json.dumps({\'name\':\'Generated package lifecycle\',\'argv\':[\'python3\',str(receiver)]})],text=True))[\'result\']\nPath(\'/tmp/ruwac-rpm-test/session.json\').write_text(json.dumps(reply))\nsubprocess.run(base+[\'rpc\',\'pane.wait\',\'--params\',json.dumps({\'pane\':reply[\'pane\'],\'contains\':\'GENERATED_READY\',\'timeout_ms\':5000})],check=True)\nCREATE\ndnf -y install /packages/*.rpm\n/usr/bin/ruwac --version\n/usr/bin/ruwac-desktop --version\nrpm -q gtk4 libadwaita python3-pillow tesseract-langpack-eng\npython3 - <<\'UPGRADE\'\nimport subprocess,json,os,time\nfrom pathlib import Path\np=Path(\'/tmp/ruwac-rpm-test\');s=json.loads((p/\'session.json\').read_text());base=[\'/usr/bin/ruwac\',\'--socket\',str(p/\'control.sock\')]\nresponse=subprocess.run([\'/usr/bin/ruwac\',\'update\',\'--auto-install\',\'status\'],capture_output=True,text=True,check=True)\nassert json.loads(response.stdout)[\'distribution\']==\'rpm\'\nrefused=subprocess.run([\'/usr/bin/ruwac\',\'update\',\'--testing-bundles\',\'on\'],capture_output=True,text=True)\nassert refused.returncode!=0 and \'managed by DNF\' in refused.stdout\nsubprocess.run(base+[\'rpc\',\'pane.sendText\',\'--params\',json.dumps({\'pane\':s[\'pane\'],\'text\':\'UPGRADED\'})],check=True)\nfor _ in range(100):\n if (p/\'input.bin\').read_bytes()==b\'UPGRADED\':break\n time.sleep(.05)\nelse:raise SystemExit(\'Upgrade lost live child input\')\nos.kill(s[\'pid\'],0)\nUPGRADE\nkill -0 "$daemon"\nsha256sum -c /tmp/ruwac-rpm-test/profile.sha256\ndnf -y downgrade /previous/*.rpm\n/usr/bin/ruwac --version\n/usr/bin/ruwac-desktop --version\npython3 - <<\'ROLLBACK\'\nimport subprocess,json,os,time\nfrom pathlib import Path\np=Path(\'/tmp/ruwac-rpm-test\');s=json.loads((p/\'session.json\').read_text());base=[\'/usr/bin/ruwac\',\'--socket\',str(p/\'control.sock\')]\nsubprocess.run(base+[\'rpc\',\'pane.sendText\',\'--params\',json.dumps({\'pane\':s[\'pane\'],\'text\':\'_ROLLED_BACK\'})],check=True)\nfor _ in range(100):\n if (p/\'input.bin\').read_bytes()==b\'UPGRADED_ROLLED_BACK\':break\n time.sleep(.05)\nelse:raise SystemExit(\'Rollback lost live child input\')\nos.kill(s[\'pid\'],0)\nROLLBACK\nkill -0 "$daemon"\nsha256sum -c /tmp/ruwac-rpm-test/profile.sha256\ndnf -y remove ruwac-desktop ruwac\nkill -0 "$daemon"\nsha256sum -c /tmp/ruwac-rpm-test/profile.sha256\n! test -e /usr/bin/ruwac\n! test -e /usr/bin/ruwac-desktop\nprintf \'%s\\n\' \'CLEAN_FEDORA_DESKTOP_DNF_INSTALL_UPGRADE_ROLLBACK_REMOVE PASS\'\n'


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--rpms', type=Path, required=True)
    p.add_argument('--previous-rpms', type=Path, required=True)
    p.add_argument('--evidence', type=Path, required=True)
    p.add_argument('--podman-root', type=Path)
    p.add_argument('--podman-runroot', type=Path)
    p.add_argument('--run-owned-fixture', action='store_true')
    a = p.parse_args()
    if not a.run_owned_fixture:
        p.error('Explicit --run-owned-fixture is required for a bounded disposable RAM mount/container')
    for directory in [a.rpms, a.previous_rpms]:
        if len(list(directory.glob('*.rpm'))) != 2:
            p.error('Both core and desktop RPMs are required for each version')
    a.evidence.mkdir(parents=True, exist_ok=True)
    r = Path(tempfile.mkdtemp(prefix='fedora-dnf-', dir=a.evidence)).resolve()
    r.chmod(0o700)
    suffix = uuid.uuid4().hex[:12]
    name, seed = 'ruwac-dnf-' + suffix, 'ruwac-dnf-seed-' + suffix
    fs = r / 'ram-root'
    fs.mkdir(mode=0o700)
    privilege = [] if os.geteuid() == 0 else ['sudo', '-n']
    podman = privilege + ['podman']
    for flag, value in [('--root', a.podman_root), ('--runroot', a.podman_runroot)]:
        if value:
            podman += [flag, str(value)]
    mounted = False
    try:
        subprocess.run(privilege + ['mount', '-t', 'tmpfs', '-o', 'size=2G,mode=700', 'tmpfs', str(fs)], check=True)
        mounted = True
        subprocess.run(podman + ['create', '--name', seed, '--pull=never', IMAGE, 'true'], capture_output=True, check=True)
        with (r / 'export.log').open('w') as log:
            exporter = subprocess.Popen(podman + ['export', seed], stdout=subprocess.PIPE, stderr=log)
            extract = subprocess.run(privilege + ['tar', '-xf', '-', '-C', str(fs), '--numeric-owner'], stdin=exporter.stdout, stderr=log)
            exporter.stdout.close()
            if exporter.wait() != 0 or extract.returncode != 0:
                raise RuntimeError('Cached official Fedora filesystem export failed')
        subprocess.run(podman + ['rm', seed], capture_output=True, check=True)
        # No host home, package database, bus, display or device tree is mounted.
        command = podman + ['run', '--rm', '--name', name, '--rootfs', '--network=host',
                            '--security-opt', 'label=disable', '--memory=2g', '--memory-swap=2g',
                            '--cpus=1', '--pids-limit=256',
                            '-v', str(a.previous_rpms.resolve()) + ':/previous:ro',
                            '-v', str(a.rpms.resolve()) + ':/packages:ro', str(fs), 'bash', '-c', BODY]
        with (r / 'dnf.log').open('w') as log:
            result = subprocess.run(command, stdout=log, stderr=subprocess.STDOUT, timeout=900)
        if result.returncode or 'CLEAN_FEDORA_DESKTOP_DNF_INSTALL_UPGRADE_ROLLBACK_REMOVE PASS' not in (r / 'dnf.log').read_text():
            raise RuntimeError('Fresh Fedora DNF lifecycle failed; retained private evidence')
        (r / 'acceptance.json').write_text(json.dumps({'status': 'PASS', 'image': IMAGE,
            'nodeps_used': False, 'actual_dependency_resolution': True,
            'desktop_runtime_packages_installed': True,
            'actual_DNF_install_upgrade_downgrade_remove': True,
            'same_daemon_and_child_survive_upgrade_rollback': True,
            'private_generated_configuration_preserved': True,
            'scope': 'Fresh Fedora44 RAM rootfs with real DNF and host kernel; not physical desktop, signing or security qualification'}, indent=2) + '\n')
        print('Fresh Fedora desktop DNF lifecycle PASS; private evidence', r)
    finally:
        for owned in [name, seed]:
            q = subprocess.run(podman + ['container', 'exists', owned], capture_output=True)
            if q.returncode == 0:
                subprocess.run(podman + ['rm', '-f', owned], capture_output=True, check=True)
        if mounted:
            subprocess.run(privilege + ['umount', str(fs)], check=True)
        fs.rmdir()
        (r / 'cleanup.json').write_text(json.dumps({'containers_removed': True, 'RAM_root_unmounted': True, 'host_package_database_unchanged': True}, indent=2) + '\n')


if __name__ == '__main__':
    main()
