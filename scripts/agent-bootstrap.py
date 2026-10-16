#!/usr/bin/env python3
"""Opt-in per-user agent installation, only when a newly created agent needs it.

No credentials are provisioned. Existing executables, aliases and functions win.
"""
import os
from pathlib import Path
import shlex
import shutil
import subprocess
import sys
import tempfile
import urllib.request

INSTALLERS = {
    'codex': ('https://chatgpt.com/codex/install.sh', 'sh'),
    'claude': ('https://claude.ai/install.sh', 'bash'),
    'kimi': ('https://code.kimi.com/kimi-code/install.sh', 'bash'),
}


def available(command, env):
    if shutil.which(command, path=env['PATH']):
        return True
    # Match the login/interactive shell used by the actual agent launcher.
    probe = subprocess.run(['bash', '-lic', 'type -t -- "$1" >/dev/null 2>&1',
                            'ruwac-agent-probe', command], env=env,
                           stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=10)
    return probe.returncode == 0


def install(provider, env):
    url, shell = INSTALLERS[provider]
    print(f'RUWAC: installing missing {provider} for this remote user from {url}', flush=True)
    # Complete the HTTPS download before executing anything. Bound both phases.
    request = urllib.request.Request(url, headers={'User-Agent': 'RUWAC-agent-bootstrap'})
    with urllib.request.urlopen(request, timeout=30) as response:
        if not response.url.startswith('https://'):
            raise ValueError('Installer requires HTTPS')
        data = response.read(1024 * 1024 + 1)
    if not data or len(data) > 1024 * 1024:
        raise ValueError('Installer download is empty or too large')
    with tempfile.TemporaryDirectory(prefix='ruwac-agent-install-') as directory:
        script = Path(directory) / 'install.sh'
        script.write_bytes(data)
        script.chmod(0o600)
        install_env = dict(env)
        if provider == 'codex':
            install_env['CODEX_NON_INTERACTIVE'] = '1'
        subprocess.run([shell, str(script)], env=install_env, check=True, timeout=600)


def main():
    if len(sys.argv) != 3 or sys.argv[1] not in INSTALLERS:
        raise ValueError('Choose a supported agent')
    provider, command = sys.argv[1:]
    if not command or len(command) > 1024 or any(ord(c) < 32 or ord(c) == 127 for c in command):
        raise ValueError('Invalid saved agent command')
    env = dict(os.environ)
    prefix = str(Path.home() / '.local/bin')
    kimi_bin = str(Path(env.get('KIMI_INSTALL_DIR', str(Path.home() / '.kimi-code'))) / 'bin')
    env['PATH'] = os.pathsep.join([prefix, kimi_bin, env.get('PATH', os.defpath)])
    # Validate the saved command; installation is keyed to the selected provider.
    words = shlex.split(command)
    if not words:
        raise ValueError('Missing agent command')
    if not available(provider, env):
        install(provider, env)
        if not available(provider, env):
            raise ValueError('Installer finished but the agent is unavailable; inspect its terminal output')
    os.execvpe('bash', ['bash', '-lic', 'export PATH="$HOME/.local/bin:${KIMI_INSTALL_DIR:-$HOME/.kimi-code}/bin:$PATH"; ' + command], env)


if __name__ == '__main__':
    try:
        main()
    except (OSError, ValueError, subprocess.SubprocessError):
        print('RUWAC agent setup failed. Inspect the installer output, then retry. '
              'Authentication remains with the agent; no login credentials were copied.', file=sys.stderr)
        sys.exit(1)
