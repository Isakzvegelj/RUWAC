# Fedora 44 GitHub updates

RUWAC checks the latest published stable GitHub release at desktop startup and daily while the window remains open. A highlighted update icon in the header opens the update dialog. Disable checks there or with `ruwac update --auto-check off`. Checks never install or restart anything automatically.

Select **Build and install update** to fetch the immutable release commit, compile both locked Rust workspaces on Fedora 44 and use the existing per-user installer. Builds take several minutes and need development packages. This is source delivery; downloadable RPMs, signed binaries and Fedora repository integration remain unfinished.

Install prerequisites once:

```sh
sudo dnf install git cargo rust gcc gcc-c++ pkgconf-pkg-config python3 \
  gtk4-devel libadwaita-devel openssl-devel sqlite-devel
```

After installing RUWAC with the updater:

```sh
ruwac update --check
ruwac update --install
ruwac update --rollback
```

For a first installation from published source:

```sh
python3 scripts/update-user.py --current 0.0.0 --install
```

The bootstrap comparison sentinel is not a product release allocation. Use `--headless` for a CLI-only machine. Existing desktop installations must retain their desktop component. `--prefix` selects another owned user installation directory; the desktop dialog uses the standard user installation.

The updater resolves annotated or lightweight tags through GitHub's authenticated HTTPS API, fetches over HTTPS and verifies the fetched commit before checkout. Changed tags, drafts, prereleases, invalid versions and source/version mismatches are rejected. Cargo uses committed lockfiles and registry checksums. This verifies GitHub origin and consistency; it is not an independent release signature or reproducible-build attestation.

Builds and caches live under the user's XDG cache directory and can consume several gigabytes. Immutable installation directories and the previous executable are retained. Close and reopen a frontend when ready to see the new UI. Updates do not restart daemons, launch windows, change profiles, retrieve credentials or touch clipboards. Rollback checks stored artifact hashes and component parity before changing the current symlink.

Preferences are private, schema checked and atomically saved. Unsafe or unknown files are preserved with a refusal. Locks prevent competing updates and installations. Failed downloads/builds cannot reach the installer. Use the terminal command for detailed build diagnostics. RUWAC does not install dependencies automatically or require root privilege to update itself.

The existing build/release scripts and controller retain publication authority. This updater creates no versions, commits, tags, releases or assets. The unfixed RSA advisory, GNOME Graphics Pipeline interoperability, physical desktop acceptance and qualified Fedora packaging remain open; a source update does not clear those gates.

## GitHub source updates — compiled package 0.2.1

Thirteen deterministic updater/installer guards pass: release filtering and numeric ordering, immutable annotated-tag resolution, changed-tag rejection, preference/foreign-file preservation, disabled-check network exclusion, locking, actual immutable installs and rollback, altered-binary refusal, missing previous installation, failed-build exclusion, update/rollback contention and a newer frontend surviving rollback. Five native GTK groups pass on an isolated Wayland compositor with synthetic widget callbacks and generated helper responses. These exercise the actual update dialog, check result, preference toggle, explicit install callback and dialog closure; GTK ticks and a real daemon-owned PTY remain responsive during the simulated slow installer. This native test does not claim a real graphical GitHub install or physical input.

Separately, the real HTTPS GitHub API and Git fetch resolved published v0.2.0 to `450a21203bdd33bd9a423e414d202b7387baa8bd`. The updater compiled both locked Rust workspaces from downloaded source and installed the headless executable, GTK executable and desktop launcher in disposable user prefixes, with clean source receipt and artifact hashing. Thus real source download/build/install is verified for both components. The first clean desktop dependency build took 10 minutes 3 seconds on this busy VM; this is an observed build time, not an idle-qualified benchmark. No unattended installation, signed binary/RPM distribution or whole-product qualification is inferred.

The stamped normal workflow runs headless protocol/persistence/copy/history/nesting regressions and 52 desktop units, then updater guards, format/lint, dependency declarations/headless isolation, native terminal/recovery/workspace/API, authenticated loopback RDP clipboard/lifecycle/display, SSH/SFTP resume/file integrity, dual-login confirmation, sidebar and user-shell regressions. Local generated fixtures and authorized external SSH/SFTP are distinguished in their receipts. Previous real Windows framebuffer evidence remains historical; no new external RDP or GDM pass is claimed by this increment.

CLI SHA-256: `673028160442e8ce15c0b803aa56d828650d98400793fa50f56101e41c5812ce`. GTK SHA-256: `1adba8ab4ed88c1282eed09e8ee229c17cced1104cd7e0b317356c2620ef0e52`. Commands: existing `build.sh --release --test --desktop` and `release.sh stable`; the controller owns the version and publication. Running windows, daemons, profiles and credentials are preserved. See [Fedora update usage](RUWAC_UPDATES.md). GNOME Graphics Pipeline, physical input, unfixed RSA advisory and qualified Fedora packages remain open.
