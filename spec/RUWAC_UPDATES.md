# Fedora 44 installation and automatic updates

Published releases include source, Fedora 44 x86_64 testing RPMs and checked per-user
binary bundles. Full security qualification and a signed Fedora repository remain
separate gates. Choose the installation mode that matches your update needs.

For system installation, download the matching core and desktop RPMs from the
same release and install them with DNF. Open /usr/bin/ruwac gui if a prior per-user
installation is also present. The RPM update dialog opens the release downloads;
system installation, updates and rollback use DNF. Unattended RPM installation is
not yet implemented. The application refuses per-user update operations that would
silently shadow a system RPM installation.

For unattended updates without local source compilation, install the release's
Fedora testing bundle using its included installer, then explicitly opt in:

```sh
"$HOME/.local/bin/ruwac" update --testing-bundles on
"$HOME/.local/bin/ruwac" update --auto-install on
```

Checked bundle delivery validates the release commit, quality/privacy jobs, archive
digest and immutable installation manifest. Published assets also have workflow
provenance; the per-user updater does not yet automatically enforce that attestation.
Running sessions are preserved. The source-build option below remains available.

Install build prerequisites:

```sh
sudo dnf install -y git cargo rust gcc gcc-c++ pkgconf-pkg-config gtk4-devel libadwaita-devel openssl-devel libssh2-devel sqlite-devel python3 python3-pillow tesseract tesseract-langpack-eng
```

Fetch the latest published source and run its bootstrap installer:

```sh
set -e
RUWAC_INSTALL_SOURCE="$(mktemp -d)"
RUWAC_INSTALL_TAG="$(python3 -c 'import json,urllib.request; print(json.load(urllib.request.urlopen("https://api.github.com/repos/w3fans/RUWAC/releases/latest"))["tag_name"])')"
git clone --depth 1 --branch "$RUWAC_INSTALL_TAG" https://github.com/w3fans/RUWAC.git "$RUWAC_INSTALL_SOURCE"
python3 "$RUWAC_INSTALL_SOURCE/scripts/update-user.py" --bootstrap
"$HOME/.local/bin/ruwac-desktop"
```

Bootstrap installs the latest stable source release under the current user's
installation prefix. It verifies the release tag's commit before checkout, uses
locked dependencies, retains previous installations and refuses downgrades. It does
not require SSH access from a developer. The initial build can take several minutes.
`--headless` installs only the terminal engine and CLI.

Enable unattended installation in the desktop's Updates dialog, or run:

```sh
"$HOME/.local/bin/ruwac" update --auto-install on
"$HOME/.local/bin/ruwac" update --auto-install status
```

This explicit opt-in creates a per-user systemd timer. While the user manager is
running, it checks after activation and every six hours, with a randomized delay.
It compiles eligible updates in the private user cache and installs them atomically.
Unattended installation waits for successful source quality and privacy jobs on the
exact release commit. The separate dependency-security advisory remains unsuppressed;
source-testing delivery is distinct from qualified binary security/distribution.

Existing windows, RDP connections and daemon sessions keep running. The next
application restart uses the installed update. Closing the frontend preserves
persistent terminal sessions. Existing headless installations remain headless.

```sh
"$HOME/.local/bin/ruwac" update --auto-install off
"$HOME/.local/bin/ruwac" update --rollback
```

Opt-out prevents an in-progress automatic build from installing after the opt-out
has committed. Edited or unrelated unit files are preserved. If the user manager
cannot disable the timer, the saved opt-out still prevents automatic installation.
Manual installation and reversible rollback retain their existing lock and immutable
artifact checks. A successful rollback disables unattended installation before
switching versions, preventing the next timer expiry from undoing recovery. The
timer may remain scheduled, but disabled checks perform no network or installation;
re-enable installation explicitly after resolving the regression. Rollback works
without a running user service manager and preserves notification preferences.
Automatic update checks and automatic installation are independent
settings; disabling notifications does not implicitly revoke installation consent.

Implementation evidence: portable updater fault tests, real expiry of uniquely
named disposable user-manager units with real GitHub release/commit checks, and actual
GTK dialog callbacks on an isolated compositor. The scheduler fixture exercises an
up-to-date installation, not a fresh source-compilation cycle. Compiler and immutable
installation behavior have separate acceptance gates. Physical desktop input and
unavailable external machines are not claimed by these tests.

Timer semantics follow the upstream [systemd timer documentation](https://github.com/systemd/systemd/blob/main/man/systemd.timer.xml).
