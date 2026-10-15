#!/usr/bin/env python3
"""Embedded SSH launcher: create once, then share the named persistent session.

Only the session identity is reused. A later command preference never sends
input to, replaces, or restarts a live process. No agent account is probed.
"""
import fcntl
import json
import os
from pathlib import Path
import shutil
import stat
import subprocess
import sys
import time


class LaunchError(Exception):
    """A fixed, safe-to-display launcher diagnostic."""


def main():
    binary, socket, name, command = sys.argv[1:]
    if name not in ('codex', 'claude'):
        raise LaunchError('Choose a supported remote agent')
    exe = shutil.which(binary)
    if not exe and binary == 'ruwac':
        candidate = os.path.expanduser('~/.local/bin/ruwac')
        if os.path.isfile(candidate) and os.access(candidate, os.X_OK):
            exe = candidate
    if not exe:
        raise LaunchError('Install the RUWAC CLI on this machine before launching a persistent agent.')
    # Use precisely the CLI default, including an explicit remote environment override.
    if not socket:
        runtime = os.environ.get('XDG_RUNTIME_DIR')
        socket = os.environ.get('RUWAC_SOCKET',
                                f'{runtime}/ruwac/control.sock' if runtime is not None
                                else f'/tmp/ruwac-{os.geteuid()}/control.sock')
    socket = os.path.abspath(socket)
    cli = [exe, '--socket', socket]
    subprocess.run(cli + ['start'], check=True, stdout=subprocess.DEVNULL, timeout=20)

    def rpc(method, params):
        reply = json.loads(subprocess.check_output(
            cli + ['rpc', method, '--params', json.dumps(params)], text=True, timeout=15))
        if 'error' in reply or 'result' not in reply:
            raise LaunchError('Remote RUWAC request failed')
        return reply['result']

    # Serialize only launchers for this agent and this daemon, including parallel
    # SSH connections. Old running daemons need no new protocol or restart.
    directory = os.open(Path(socket).parent, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW)
    try:
        info = os.fstat(directory)
        if info.st_uid != os.geteuid() or info.st_mode & 0o077:
            raise LaunchError('Remote daemon directory must be private and owned by this account')
        lock = os.open(f'.agent-{name}.lock', os.O_CREAT | os.O_RDWR | os.O_NOFOLLOW | os.O_NONBLOCK,
                       0o600, dir_fd=directory)
    finally:
        os.close(directory)
    try:
        info = os.fstat(lock)
        if (not stat.S_ISREG(info.st_mode) or info.st_uid != os.geteuid()
                or info.st_mode & 0o077 or info.st_nlink != 1):
            raise LaunchError('Unsafe remote agent launch lock')
        deadline = time.monotonic() + 20
        while True:
            try:
                fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
                break
            except BlockingIOError:
                if time.monotonic() >= deadline:
                    raise LaunchError('Another remote launch is busy; retry shortly')
                time.sleep(0.05)
        matches = [s for s in rpc('session.list', {}) if s.get('name') == name]
        if len(matches) > 1:
            raise LaunchError('Multiple sessions have this name. Use ruwac -ls and attach by session ID.')
        if matches:
            sid = matches[0]['id']
            state = 'Reattaching to'
        else:
            sid = rpc('session.create', {'name': name, 'argv': ['bash', '-lic', command]})['session']
            state = 'Created'
        if not isinstance(sid, str) or not sid.startswith('session_'):
            raise LaunchError('Remote RUWAC session identity rejected')
    finally:
        # Keep the lock inode, but never pass an open lock to the attached client.
        os.close(lock)
    print(f'{state} persistent {name} session: {sid}', flush=True)
    print('Closing this view detaches; the remote agent keeps running.', flush=True)
    os.execv(exe, cli + ['attach', '--session', sid])


if __name__ == '__main__':
    try:
        main()
    except LaunchError as error:
        print(f'Remote agent launch failed: {error}', file=sys.stderr)
        sys.exit(1)
    except (OSError, ValueError, KeyError, subprocess.SubprocessError):
        # Do not echo subprocess commands, configuration, or authentication data.
        print('Remote agent launch failed. Check RUWAC availability and the private daemon directory; '
              'use ruwac -ls to resolve duplicate session names.', file=sys.stderr)
        sys.exit(1)
