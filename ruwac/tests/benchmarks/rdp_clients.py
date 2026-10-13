#!/usr/bin/env python3
"""Owned X11/RDP fixture: compare real client pixels, never an owner's desktop.

Requires FreeRDP, Xvfb, OpenSSL, python-xlib and Pillow. Generated credentials
stay in fixture memory/stdin. This does not qualify external/physical desktops.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import select
import statistics
import subprocess
import sys
import tempfile
import time
from PIL import Image, ImageChops
from Xlib import X, display

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'integration'))
from spike import Client, eventually

p = argparse.ArgumentParser(description=__doc__)
p.add_argument('--run-owned-fixture', action='store_true', required=True)
for name in ['binary', 'gui', 'server', 'evidence']:
    p.add_argument('--' + name, type=Path, required=True)
p.add_argument('--freerdp', default='xfreerdp')
p.add_argument('--check-budget', action='store_true')
p.add_argument('--trials', type=int, default=10)
p.add_argument('--frames', type=int, default=5)
a = p.parse_args()
if not 1 <= a.trials <= 30 or not 2 <= a.frames <= 100:
    p.error('Trials must be 1..30 and frames 2..100')
if a.check_budget:
    if a.trials < 10 or a.frames < 5:
        p.error('Budget qualification needs at least ten trials and five frames')
    quota, period = Path('/sys/fs/cgroup/cpu.max').read_text().split()
    assert quota != 'max' and int(quota) == int(period), 'Budget requires one-CPU fixture'
    assert int(Path('/sys/fs/cgroup/memory.max').read_text()) == 1024**3, 'Budget requires 1 GiB fixture'
root = Path(tempfile.mkdtemp(prefix='rdp-comparison-', dir=a.evidence))
root.chmod(0o700)
env = {**os.environ, 'GTK_A11Y': 'none', 'GDK_BACKEND': 'x11', 'GSK_RENDERER': 'cairo',
       'GSETTINGS_BACKEND': 'memory', 'GTK_USE_PORTAL': '0'}
for key, name in [('HOME', 'home'), ('XDG_CONFIG_HOME', 'config'), ('XDG_DATA_HOME', 'data'), ('XDG_CACHE_HOME', 'cache')]:
    env[key] = str(root / name)
    Path(env[key]).mkdir(mode=0o700)
# Do not inherit an owner's desktop bus or accessibility bus.
env.pop('DBUS_SESSION_BUS_ADDRESS', None)
env.pop('AT_SPI_BUS_ADDRESS', None)
children, logs, records = [], [], []


def start(argv, name, **kw):
    log = (root / (name + '.log')).open('wb')
    logs.append(log)
    child = subprocess.Popen([str(x) for x in argv], env=kw.pop('env', env), stdout=log, stderr=log, **kw)
    children.append(child)
    return child


def stop(child):
    if child.poll() is None:
        child.terminate()
    try:
        child.wait(timeout=5)
    except subprocess.TimeoutExpired:
        child.kill()
        child.wait()


def process_metrics(child):
    stat = Path('/proc', str(child.pid), 'stat').read_text().rsplit(')', 1)[1].split()
    rss = next(int(line.split()[1]) for line in Path('/proc', str(child.pid), 'status').read_text().splitlines() if line.startswith('VmRSS:'))
    return (int(stat[11]) + int(stat[12])) / os.sysconf('SC_CLK_TCK'), rss


def summarize(values):
    return {'count': len(values), 'median': statistics.median(values),
            'p95': sorted(values)[max(0, __import__('math').ceil(len(values) * .95) - 1)]}


try:
    subprocess.run(['openssl', 'req', '-x509', '-newkey', 'rsa:2048', '-nodes', '-days', '1',
                    '-subj', '/CN=generated-loopback', '-addext', 'subjectAltName=IP:127.0.0.1',
                    '-keyout', str(root / 'key.pem'), '-out', str(root / 'cert.pem')],
                   check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    (root / 'key.pem').chmod(0o600)
    fingerprint = hashlib.sha256(subprocess.check_output(['openssl', 'x509', '-in', str(root / 'cert.pem'), '-outform', 'DER'])).hexdigest()
    read_fd, write_fd = os.pipe()
    xvfb = start(['Xvfb', '-displayfd', write_fd, '-screen', '0', '1280x900x24', '-nolisten', 'tcp'], 'xvfb', pass_fds=(write_fd,))
    os.close(write_fd)
    assert select.select([read_fd], [], [], 10)[0], 'Owned Xvfb display allocation timed out'
    with os.fdopen(read_fd) as pipe:
        number = pipe.readline().strip()
    assert number.isdigit(), 'Owned Xvfb did not provide a display'
    env['DISPLAY'] = ':' + number
    d = display.Display(env['DISPLAY'])
    screen = d.screen().root
    server = start([a.server, root], 'server', env={**env, 'RUWAC_FIXTURE_FRAME_MARKERS': '1'})
    eventually(lambda: (root / 'server-ready.json').exists(), 10)
    port = json.loads((root / 'server-ready.json').read_text())['ports'][0]
    sock = root / 'control.sock'
    daemon = start([a.binary, '--socket', sock, 'serve'], 'daemon')
    eventually(sock.exists)
    with Client(sock) as c:
        session = c.call('session.create', {'name': 'Generated comparator', 'argv': ['/bin/cat']})
    for trial in range(a.trials):
        # Alternate order to expose warm-up/order effects.
        for kind in (['ruwac', 'freerdp'] if trial % 2 == 0 else ['freerdp', 'ruwac']):
            gui_root = root / (kind + '-' + str(trial))
            gui_root.mkdir(mode=0o700)
            shutil.copy2(root / 'cert.pem', gui_root / 'cert.pem')
            seq = [0]
            def command(action, **kw):
                seq[0] += 1
                n = seq[0]
                tmp = gui_root / 'request.tmp'
                tmp.write_text(json.dumps({'seq': n, 'action': action, **kw}))
                tmp.replace(gui_root / 'gui-request.json')
                def read():
                    try:
                        result = json.loads((gui_root / 'gui-response.json').read_text())
                        return result['result'] if result['seq'] == n else None
                    except (OSError, ValueError):
                        return None
                return eventually(read, 10)
            started_ns = time.monotonic_ns()
            if kind == 'ruwac':
                client = start([a.gui, '--new-instance', '--socket', sock, '--session', session['session'], '--fixture-dir', gui_root], kind + '-' + str(trial))
                command('inspect')
                command('rdp-open', port=port)
                command('rdp-display', mode='native', zoom=1.0)
            else:
                client = start([a.freerdp, '/v:127.0.0.1:' + str(port), '/u:fixture', '/from-stdin:force',
                                '/cert:fingerprint:sha256:' + fingerprint, '/size:640x480', '/sec:nla', '/log-level:WARN'],
                               kind + '-' + str(trial), stdin=subprocess.PIPE)
                client.stdin.write(b'generated-local-only\n')
                client.stdin.close()
            latency, seen, roi, first_ns = [], set(), None, None
            cpu_before, _ = process_metrics(client)
            cpu_started = time.monotonic()
            while len(latency) < a.frames and time.monotonic_ns() - started_ns < 30_000_000_000:
                assert client.poll() is None, 'Client exited before observed frames'
                if roi is None:
                    capture = screen.get_image(0, 0, 1280, 900, X.ZPixmap, 0xffffffff)
                    observed_ns = time.monotonic_ns()
                    image = Image.frombytes('RGB', (1280, 900), capture.data, 'raw', 'BGRX')
                    colors = [(n, color) for n, color in image.getcolors(1280 * 900) if color[2] == 211 and color[0] + color[1] * 256 > 0]
                    if not colors or max(colors)[0] < 10000:
                        time.sleep(.01)
                        continue
                    _, color = max(colors)
                    masks = [channel.point(lambda value, expected=expected: 255 if value == expected else 0) for channel, expected in zip(image.split(), color)]
                    box = ImageChops.multiply(ImageChops.multiply(masks[0], masks[1]), masks[2]).getbbox()
                    roi = ((box[0] + box[2]) // 2, (box[1] + box[3]) // 2)
                else:
                    data = screen.get_image(*roi, 1, 1, X.ZPixmap, 0xffffffff).data
                    observed_ns = time.monotonic_ns()
                    color = (data[2], data[1], data[0])
                sequence = color[0] + color[1] * 256
                if color[2] == 211 and sequence not in seen:
                    try:
                        stamp = json.loads((root / 'frame-marker-0.json').read_text())
                    except (OSError, ValueError):
                        continue
                    if stamp['sequence'] == sequence and started_ns <= stamp['generated_ns'] <= observed_ns:
                        assert (stamp['width'], stamp['height']) == (640, 480), 'Comparators must use identical framebuffer size'
                        if first_ns is None:
                            first_ns = observed_ns
                        latency.append((observed_ns - stamp['generated_ns']) / 1e6)
                        seen.add(sequence)
                time.sleep(.005)
            assert len(latency) == a.frames, 'Insufficient observed timestamped framebuffer updates'
            cpu_after, rss = process_metrics(client)
            accepted_ns = json.loads((root / 'accepted-marker-0.json').read_text())['accepted_ns']
            assert started_ns <= accepted_ns <= first_ns, 'Transport timestamp does not belong to this trial'
            record = {'tcp_accept_to_visible_frame_ms': (first_ns - accepted_ns) / 1e6, 'local_scale': 'native', 'client': kind, 'trial': trial, 'first_visible_frame_ms': (first_ns - started_ns) / 1e6,
                      'generation_to_X11_readback_ms': latency, 'rss_kib': rss,
                      'client_cpu_percent_one_core': (cpu_after - cpu_before) / (time.monotonic() - cpu_started) * 100,
                      'framebuffer': [640, 480], 'TLS_and_CredSSP': True}
            if kind == 'ruwac':
                state = command('rdp-inspect', rdp=0)
                assert state['size'] == [640, 480], state
                record['native_viewport'] = state['viewport']
                record['independent_daemon_rss_kib'] = process_metrics(daemon)[1]
            records.append(record)
            print(kind, trial, 'visible frames', len(latency), flush=True)
            stop(client)
    report = {'records': records, 'summary': {kind: {
        'tcp_accept_to_visible_frame_ms': summarize([r['tcp_accept_to_visible_frame_ms'] for r in records if r['client'] == kind]),
        'first_visible_frame_ms': summarize([r['first_visible_frame_ms'] for r in records if r['client'] == kind]),
        'generation_to_X11_readback_ms': summarize([x for r in records if r['client'] == kind for x in r['generation_to_X11_readback_ms']]),
        'rss_kib': summarize([r['rss_kib'] for r in records if r['client'] == kind]),
        'client_cpu_percent_one_core': summarize([r['client_cpu_percent_one_core'] for r in records if r['client'] == kind])} for kind in ['ruwac', 'freerdp']},
        'artifacts': {name: hashlib.sha256(path.read_bytes()).hexdigest() for name, path in [('core', a.binary), ('desktop', a.gui), ('fixture', a.server)]},
        'scope': 'Actual authenticated same-loopback-endpoint clients, identical 640x480 frames, owned X11 pixels and software Cairo rendering. Includes server encoding, transport, decode, render and observer scheduling. Cold UI/client launch included and TCP-accept-to-visible-frame reported separately; both clients use native scale. Daemon setup excluded and RSS reported separately. Alternating serial trials; fresh processes with shared filesystem/font caches, not cold system-cache tests. No physical scanout, WAN, Windows/GNOME or multi-connection claim. No security/stability qualification.'}
    budget = report['summary']['ruwac']
    report['budget_checks'] = {'tcp_first_frame_median': budget['tcp_accept_to_visible_frame_ms']['median'] <= 250,
                              'paint_latency_p95': budget['generation_to_X11_readback_ms']['p95'] <= 150,
                              'GUI_RSS_p95': budget['rss_kib']['p95'] <= 256 * 1024,
                              'launch_first_frame_median': budget['first_visible_frame_ms']['median'] <= 1200}
    report['budget_enforced'] = a.check_budget
    (root / 'receipt.json').write_text(json.dumps(report, indent=2) + '\n')
    (a.evidence / 'rdp-comparison.json').write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(report['summary']), flush=True)
    if a.check_budget:
        assert all(report['budget_checks'].values()), 'Measured fixture performance exceeds its empirical budget'
finally:
    for child in reversed(children):
        stop(child)
    for log in logs:
        log.close()
