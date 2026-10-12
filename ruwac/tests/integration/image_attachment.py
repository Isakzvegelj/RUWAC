#!/usr/bin/env python3
"""Attachment refusal boundaries on real owned PTYs. No provider account required."""
import argparse, json, os, subprocess, tempfile, time
from pathlib import Path
from spike import Client

p = argparse.ArgumentParser()
p.add_argument('--binary', required=True)
p.add_argument('--evidence', type=Path, required=True)
a = p.parse_args()
root = Path(tempfile.mkdtemp(prefix='image-attachment-', dir=a.evidence))
root.chmod(0o700)
sock = root / 'control.sock'
log = (root / 'daemon.log').open('wb')
daemon = subprocess.Popen([a.binary, '--socket', str(sock), 'serve'], stdout=log, stderr=log)
try:
    for _ in range(100):
        if sock.exists():
            break
        time.sleep(.05)
    with Client(sock) as c:
        # A forged title and enabled bracketed paste must not qualify a plain process.
        session = c.call('session.create', {'name':'Attachment refusal', 'argv':['python3', '-c', 'import time; print("\\033[?2004hOpenAI Codex",flush=True); time.sleep(60)']})
        c.call('pane.wait', {'pane':session['pane'], 'contains':'OpenAI Codex', 'timeout_ms':5000})
        image = root / 'owned.png'
        image.write_bytes(b'\x89PNG\r\n\x1a\n')
        image.chmod(0o600)
        job = c.call('job.query', {'pane':session['pane']})
        params = {'pane':session['pane'], 'path':str(image), 'foreground_pid':job['foreground']['pid']}
        c.call('pane.attachImage', params, error='STALE_TARGET')
        c.call('pane.attachImage', {**params, 'foreground_pid':1}, error='STALE_TARGET')
        c.call('pane.attachImage', {**params, 'path':str(root/'line\nbreak.png')}, error='INVALID_PATH')
        image.chmod(0o644)
        c.call('pane.attachImage', params, error='INVALID_IMAGE')
        image.chmod(0o600)
        alias = root / 'alias.png'
        alias.symlink_to(image)
        c.call('pane.attachImage', {**params, 'path':str(alias)}, error='INVALID_IMAGE')
        alias.unlink()
        os.link(image, alias)
        c.call('pane.attachImage', params, error='INVALID_IMAGE')
        alias.unlink()
        image.write_bytes(b'not a PNG')
        c.call('pane.attachImage', params, error='INVALID_IMAGE')
        result = {'refusals':7, 'ordinary_process_with_forged_title_refused':True, 'real_owned_PTY':True, 'provider_consumption':'separate authenticated acceptance required'}
        (root/'receipt.json').write_text(json.dumps(result, indent=2)+'\n')
        print('IMAGE_ATTACHMENT_GUARDS PASS', root)
finally:
    daemon.terminate()
    daemon.wait(timeout=10)
    log.close()
