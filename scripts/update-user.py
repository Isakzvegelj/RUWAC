#!/usr/bin/env python3
"""GitHub source updates for per-user Fedora installations; never publishes a release."""
import argparse
import fcntl
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import stat
import subprocess
import tempfile
import tomllib
import urllib.request
import urllib.parse

API = 'https://api.github.com/repos/w3fans/RUWAC'
REPOSITORY = 'https://github.com/w3fans/RUWAC.git'
VERSION = re.compile(r'(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\Z')
SHA = re.compile(r'[0-9a-f]{40}\Z')


def version(value):
    if not isinstance(value, str) or not VERSION.fullmatch(value):
        raise ValueError('Invalid numeric release version')
    return tuple(map(int, value.split('.')))


def private_directory(path):
    path.mkdir(mode=0o700, parents=True, exist_ok=True)
    info = path.lstat()
    if not stat.S_ISDIR(info.st_mode) or info.st_uid != os.geteuid() or info.st_mode & 0o077:
        raise ValueError('Update state directory must be private and owned by this user')
    return path


def api(path):
    request = urllib.request.Request(API + path, headers={
        'Accept': 'application/vnd.github+json', 'User-Agent': 'RUWAC-updater',
        'Cache-Control': 'no-cache'})
    with urllib.request.urlopen(request, timeout=20) as response:
        if urllib.parse.urlparse(response.url).hostname != 'api.github.com':
            raise ValueError('Unexpected GitHub API redirect')
        data = response.read(2 * 1024 * 1024 + 1)
        if len(data) > 2 * 1024 * 1024:
            raise ValueError('GitHub response exceeds limit')
        return json.loads(data)


def latest(current):
    version(current)
    release = api('/releases/latest')
    tag = release.get('tag_name', '')
    if not isinstance(tag, str) or not tag.startswith('v'):
        raise ValueError('Invalid GitHub release tag')
    newer = version(tag[1:]) > version(current)
    url = 'https://github.com/w3fans/RUWAC/releases/tag/' + tag
    if release.get('draft') is not False or release.get('prerelease') is not False or release.get('html_url') != url:
        raise ValueError('Expected a published stable RUWAC source release')
    ref = api('/git/ref/tags/' + tag)['object']
    if not isinstance(ref.get('sha'), str) or not SHA.fullmatch(ref['sha']):
        raise ValueError('Invalid release reference')
    if ref['type'] == 'tag':
        ref = api('/git/tags/' + ref['sha'])['object']
    if ref['type'] != 'commit' or not SHA.fullmatch(ref['sha']):
        raise ValueError('Release does not identify an immutable commit')
    return {'current': current, 'latest': tag[1:], 'tag': tag, 'commit': ref['sha'],
            'available': newer, 'url': url, 'delivery': 'source-build'}


def settings_path():
    root = Path(os.environ.get('XDG_CONFIG_HOME', Path.home() / '.config')) / 'RUWAC'
    return private_directory(root) / 'updates.json'


def automatic_setting(value=None):
    path = settings_path()
    enabled = True
    if path.exists() or path.is_symlink():
        info = path.lstat()
        if not stat.S_ISREG(info.st_mode) or info.st_uid != os.geteuid() or info.st_mode & 0o077 or info.st_size > 4096:
            raise ValueError('Unsafe update preferences; preserve and repair them')
        data = json.loads(path.read_text())
        if data.get('schema') != 1 or not isinstance(data.get('automatic_check'), bool):
            raise ValueError('Unknown update preferences; preserve and repair them')
        enabled = data['automatic_check']
    if value is not None:
        fd, name = tempfile.mkstemp(prefix='.updates-', dir=path.parent)
        try:
            with os.fdopen(fd, 'w') as stream:
                json.dump({'schema': 1, 'automatic_check': value}, stream)
                stream.write('\n'); stream.flush(); os.fsync(stream.fileno())
            Path(name).replace(path)
        finally:
            Path(name).unlink(missing_ok=True)
        enabled = value
    return enabled


def run(command, *, cwd=None, env=None):
    subprocess.run(command, cwd=cwd, env=env, check=True, stdout=2, stderr=2)


def checkout(release, root, repository=REPOSITORY):
    # No global Git aliases, URL rewrites, hooks, credentials or SSH transport.
    env = dict(os.environ, GIT_CONFIG_NOSYSTEM='1', GIT_CONFIG_GLOBAL=os.devnull,
               GIT_TERMINAL_PROMPT='0')
    for key in list(env):
        if key.startswith('GIT_CONFIG_') and key not in {'GIT_CONFIG_NOSYSTEM', 'GIT_CONFIG_GLOBAL'}:
            del env[key]
        elif key.startswith('GIT_') and key not in {'GIT_CONFIG_NOSYSTEM', 'GIT_CONFIG_GLOBAL', 'GIT_TERMINAL_PROMPT'}:
            del env[key]
    run(['git', '-c', 'init.templateDir=', 'init', str(root)], env=env)
    run(['git', '-c', 'core.hooksPath=/dev/null', '-C', str(root), 'fetch', '--depth=1',
         repository, 'refs/tags/' + release['tag']], env=env)
    found = subprocess.check_output(['git', '-C', str(root), 'rev-parse', 'FETCH_HEAD^{commit}'], env=env, text=True).strip()
    if found != release['commit']:
        raise ValueError('Release tag changed during download; nothing installed')
    run(['git', '-c', 'core.hooksPath=/dev/null', '-C', str(root), 'checkout', '--detach', found], env=env)
    for path in ['ruwac/Cargo.toml', 'ruwac/desktop/Cargo.toml']:
        data = tomllib.loads((root / path).read_text())
        package = data.get('workspace', {}).get('package', data.get('package', {}))
        if package.get('version') != release['latest']:
            raise ValueError('Source version differs from GitHub release')


def locked_file(path):
    fd = os.open(path, os.O_CREAT | os.O_RDWR | os.O_NOFOLLOW, 0o600)
    info = os.fstat(fd)
    if not stat.S_ISREG(info.st_mode) or info.st_uid != os.geteuid() or info.st_nlink != 1 or info.st_mode & 0o077:
        os.close(fd)
        raise ValueError('Unsafe update lock')
    try:
        fcntl.flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
    except Exception:
        os.close(fd)
        raise ValueError('Another RUWAC update or installation is active')
    return fd


def installed_version(prefix, fallback):
    for path in [prefix, prefix / 'lib', prefix / 'lib/ruwac', prefix / 'lib/ruwac/releases']:
        if path.exists() or path.is_symlink():
            if path.is_symlink() or not path.is_dir() or path.stat().st_uid != os.geteuid():
                raise ValueError('Unsafe per-user installation path')
    current = prefix / 'lib/ruwac/current'
    if not current.exists() and not current.is_symlink():
        return fallback
    if not current.is_symlink():
        raise ValueError('Unsafe current installation link')
    target = str(current.readlink())
    if not target.startswith('releases/') or len(Path(target).parts) != 2:
        raise ValueError('Unsafe current release target')
    installed = Path(target).name
    version(installed)
    receipt = current / 'installation.json'
    info = receipt.lstat()
    if (current.parent / target).is_symlink() or not stat.S_ISREG(info.st_mode) or info.st_uid != os.geteuid() or info.st_mode & 0o077 or info.st_size > 4096:
        raise ValueError('Unsafe current installation receipt')
    if json.loads(receipt.read_text())['version'] != installed:
        raise ValueError('Current installation receipt mismatch')
    return installed


def rollback(prefix):
    base = prefix / 'lib/ruwac'
    if base.is_symlink() or not base.is_dir() or base.stat().st_uid != os.geteuid():
        raise ValueError('No owned per-user installation')
    lock = locked_file(base / '.install.lock')
    try:
        links = []
        for name in ['current', 'previous']:
            link = base / name
            if not link.is_symlink():
                raise ValueError('No reversible previous installation')
            target = str(link.readlink())
            if not target.startswith('releases/') or len(Path(target).parts) != 2:
                raise ValueError('Unsafe release link')
            version(Path(target).name)
            folder = base / target
            if (base / 'releases').is_symlink() or folder.is_symlink() or folder.stat().st_uid != os.geteuid():
                raise ValueError('Unsafe release directory')
            receipt = folder / 'installation.json'
            info = receipt.lstat()
            if not stat.S_ISREG(info.st_mode) or info.st_uid != os.geteuid() or info.st_mode & 0o077 or info.st_size > 4096:
                raise ValueError('Unsafe installation receipt')
            manifest = json.loads(receipt.read_text())
            if manifest['version'] != Path(target).name or set(manifest['artifacts']) not in [{'ruwac'}, {'ruwac', 'ruwac-desktop'}]:
                raise ValueError('Invalid installation receipt')
            for name, digest in manifest['artifacts'].items():
                binary = folder / name
                info = binary.lstat()
                if not stat.S_ISREG(info.st_mode) or info.st_uid != os.geteuid() or info.st_mode & 0o022 or hashlib.sha256(binary.read_bytes()).hexdigest() != digest:
                    raise ValueError('Installed artifact changed; refusing rollback')
            links.append((target, manifest))
        if set(links[0][1]['artifacts']) != set(links[1][1]['artifacts']):
            raise ValueError('Rollback must retain the same installed components')
        for name, target in [('previous', links[0][0]), ('current', links[1][0])]:
            temp = base / ('.' + name + '-update')
            if temp.exists() or temp.is_symlink():
                raise ValueError('Rollback conflict; preserve existing paths')
        for name, target in [('previous', links[0][0]), ('current', links[1][0])]:
            temp = base / ('.' + name + '-update')
            temp.symlink_to(target); temp.replace(base / name)
        return {'installed': links[1][1]['version'], 'previous': links[0][1]['version'], 'running_sessions': 'preserved'}
    finally:
        os.close(lock)


def install(release, prefix, cache, desktop=True):
    os_release = Path('/etc/os-release').read_text()
    os_info = dict(line.split('=', 1) for line in os_release.splitlines() if '=' in line)
    if os_info.get('ID', '').strip('"') != 'fedora' or os_info.get('VERSION_ID', '').strip('"') != '44':
        raise ValueError('This source updater is currently validated for Fedora 44')
    for tool in ['git', 'cargo', 'cc', 'pkg-config', 'python3']:
        if not shutil.which(tool):
            raise ValueError('Missing build dependency: ' + tool + '; see Fedora update instructions')
    if desktop:
        # libssh2-sys already builds its bundled library when system headers are absent.
        for package in ['gtk4', 'libadwaita-1', 'openssl', 'sqlite3']:
            run(['pkg-config', '--exists', package])
    lock = locked_file(cache / '.update.lock')
    root = Path(tempfile.mkdtemp(prefix='source-', dir=cache))
    try:
        checkout(release, root)
        target = private_directory(cache / 'target')
        cargo = private_directory(cache / 'cargo')
        (root / 'target').symlink_to(target)
        env = dict(os.environ, CARGO_TARGET_DIR=str(target), CARGO_HOME=str(cargo), CARGO_BUILD_JOBS='2')
        # User installation builds published source, not the private publication controller.
        for manifest in ['ruwac/Cargo.toml'] + (['ruwac/desktop/Cargo.toml'] if desktop else []):
            run(['cargo', 'build', '--release', '--locked', '--manifest-path', str(root / manifest)], env=env)
        if version(installed_version(prefix, '0.0.0')) > version(release['latest']):
            raise ValueError('A newer installation appeared during this build; refusing a downgrade')
        args = ['python3', str(root / 'scripts/install-user.py'), '--build-root', str(root), '--prefix', str(prefix)]
        if desktop:
            args.append('--desktop')
        run(args)
        return dict(release, installed=release['latest'], running_sessions='preserved')
    finally:
        shutil.rmtree(root)
        os.close(lock)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    modes = parser.add_mutually_exclusive_group(required=True)
    modes.add_argument('--check', action='store_true')
    modes.add_argument('--check-auto', action='store_true')
    modes.add_argument('--install', action='store_true')
    modes.add_argument('--rollback', action='store_true')
    modes.add_argument('--auto-check', choices=['on', 'off', 'status'])
    parser.add_argument('--current', required=True)
    parser.add_argument('--prefix', type=Path, default=Path.home() / '.local')
    parser.add_argument('--headless', action='store_true')
    args = parser.parse_args()
    version(args.current)
    prefix = args.prefix.absolute()
    if args.auto_check:
        result = {'automatic_check': automatic_setting({'on': True, 'off': False}.get(args.auto_check))}
    elif args.check_auto and not automatic_setting():
        result = {'automatic_check': False, 'available': False}
    elif args.rollback:
        installed_version(prefix, args.current)
        cache = private_directory(Path(os.environ.get('XDG_CACHE_HOME', Path.home() / '.cache')) / 'RUWAC/updates')
        lock = locked_file(cache / '.update.lock')
        try:
            result = rollback(prefix)
        finally:
            os.close(lock)
    else:
        result = latest(installed_version(prefix, args.current))
        result['running'] = args.current
        if args.install and result['available']:
            cache = private_directory(Path(os.environ.get('XDG_CACHE_HOME', Path.home() / '.cache')) / 'RUWAC/updates')
            result = install(result, prefix, cache, not args.headless)
    print(json.dumps(result))


if __name__ == '__main__':
    try:
        main()
    except Exception as error:
        print(json.dumps({'error': str(error)}))
        raise SystemExit(1)
