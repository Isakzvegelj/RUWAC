#!/usr/bin/env python3
"""Synthetic SSH peers test fail-closed shell discovery; no remote-login claim."""
import argparse
import json
import os
from pathlib import Path
import subprocess
import tempfile

p = argparse.ArgumentParser()
p.add_argument('--binary', type=Path, required=True)
p.add_argument('--evidence', type=Path, required=True)
a = p.parse_args()
root = Path(tempfile.mkdtemp(prefix='cli-shell-contract-', dir=a.evidence))
root.chmod(0o700)
peer = root / 'ssh'
peer.write_text('''#!/usr/bin/env python3
import json,os,shlex,sys
from pathlib import Path
args=shlex.split(sys.argv[-1]);mode=os.environ['GENERATED_MODE']
with Path(os.environ['GENERATED_RECEIPT']).open('a')as f:f.write(args[-1]+'\\n')
if args[-1]=='shell-argv':
 if mode=='old-endpoint':print('usage: old client');sys.exit(1)
 v={'version':0,'result':{'argv':['/remote-only/selected $(literal) shell','-l']}}
 if mode=='relative':v['result']['argv'][0]='relative-shell'
 if mode=='control':v['result']['argv'][0]='/shell\\nextra'
 if mode=='extra-args':v['result']['argv']=['/bin/sh','-c','extra']
 if mode=='wrong-option':v['result']['argv'][1]='-c'
 if mode=='wrong-version':v['version']=99
 if mode=='missing':v['result']={}
 if mode=='error':v['error']={'code':'DENIED'}
 if mode=='long-path':v['result']['argv'][0]='/'+'x'*4096
 print(json.dumps(v))
else:
 q=json.load(sys.stdin)
 assert q['method']=='window.create'
 assert q['params']['argv']==['/usr/bin/env','-u','NO_COLOR','--','/remote-only/selected $(literal) shell','-l']
 print(json.dumps({'version':0,'id':q['id'],'result':{'generated':True}}))
''')
peer.chmod(0o700)
receipt = root / 'calls.txt'
env = {**os.environ, 'PATH': str(root) + os.pathsep + os.environ['PATH'],
       'SHELL': '/generated/nonexistent-local-shell', 'GENERATED_RECEIPT': str(receipt)}
checks = []
for mode in ['ok', 'relative', 'control', 'extra-args', 'wrong-option', 'wrong-version',
             'missing', 'error', 'long-path', 'old-endpoint']:
    receipt.unlink(missing_ok=True)
    result = subprocess.run([str(a.binary.resolve()), '--host', 'generated-endpoint',
                             'window', 'create', '--session', 'session_' + 'a' * 32, '--json'],
                            env={**env, 'GENERATED_MODE': mode}, capture_output=True, timeout=10)
    assert (result.returncode == 0) == (mode == 'ok'), (mode, result.stdout, result.stderr)
    assert receipt.read_text().splitlines() == (['shell-argv', 'request'] if mode == 'ok' else ['shell-argv'])
    if mode == 'old-endpoint':
        assert b'update the remote RUWAC CLI' in result.stderr
    checks.append({'id': mode, 'status': 'PASS'})
(root / 'acceptance.json').write_text(json.dumps({'checks': checks, 'scope': 'Synthetic SSH subprocesses only'}, indent=2) + '\n')
print('CLI_SHELL_CONTRACT PASS', len(checks), 'groups')
