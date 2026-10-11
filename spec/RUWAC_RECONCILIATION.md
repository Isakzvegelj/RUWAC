# RUWAC implementation reconciliation

## Current authenticated host CLI and discovery — 2026-10-11

`ruwac --host SSH_HOST` now sends named commands, raw RPC and JSON stdin requests through authenticated OpenSSH. `--remote-binary` selects the endpoint executable; an explicit `--socket` selects the endpoint socket. Without it, the remote CLI chooses its own private default: the local RUWAC_SOCKET is not forwarded. Existing SSH config and keys are used with BatchMode and strict host-key checking; agent and configured port forwarding are disabled. Request data travels as JSON on stdin, never as a remote shell argument. Executable/socket arguments are individually shell-quoted. Destination/option validation rejects malformed hosts, duplicate globals and remote desktop actions.

`--host HOST start` starts/reuses the remote daemon only when explicitly requested. `--host HOST attach --session ID` opens a real SSH PTY, retaining daemon sessions after detach. Control does not implicitly start a daemon. Remote reply version/correlation and 1 MiB bounds are checked. Control has a 70-second response deadline and a two-second post-EOF exit bound; failure terminates/reaps the SSH process. Interactive attachment is intentionally governed by the user’s session lifetime.

`describe` is an additive read-only daemon method exposing schema, capabilities, version and transport boundaries. The CLI probes the schema first and can aggregate existing read-only methods for an older live daemon, returning native_describe=false without inventing capabilities or restarting its processes. Discovery does not change daemon revision. The daemon/CLI remain independent of GTK, IronRDP and libssh2.


## Current individual view management — 2026-10-11

**Views…** opens a native dialog for the selected mixed workspace. Each leaf offers **Move to own tab** and **Close view**. Removing a leaf collapses its divider and keeps the sibling widget/tree alive. Moving preserves the underlying connection; closing a terminal detaches its presentation while daemon programs keep running, closing RDP disconnects only that connection, and closing Files cancels its work and releases its own lease. Closing the last leaf removes the empty tab. Dialog errors remain visible if a view disappeared. The dialog takes local focus before opening; remote clipboard ownership cannot follow it.

Connection registries remove only the selected leaf. Profile deletion protection survives a move and remains until the last associated view closes. Workspace schema 2 records the resulting topology without a schema change; no daemon protocol, engine or dependency replacement was introduced. Existing terminal splits inside a session remain independent of mixed GUI leaves.


## Current mixed workspace increment — 2026-10-11

Continues the published v0.1.5 baseline with existing history, daemon, CLI and dependencies. No replacement engine, market evaluation, shared controller edits or manual versions.

Native **Split tabs…** joins existing terminal, RDP and Files views into nested horizontal/vertical GTK Paned trees. **Separate views** returns the leaves to tabs without replacing terminal PTYs or RDP transports. Profile deletion leases survive reparenting. RDP controls now live in a popup, leaving room for the framebuffer in mixed splits.

Clipboard ownership follows the focused terminal canvas or RDP canvas, rather than the enclosing tab. Files and local controls clear remote ownership and release held RDP input. Stage clipboard PNG remembers the focused terminal across the toolbar click, refocuses it before staging, and keeps a visible success/error message. Inactive RDP offers remain isolated.

Private workspace schema 2 records all tab kinds, nested split orientations/ratios, selected tab, nonsecret RDP configuration and Files directories. A 750 ms checkpoint and final close save support GUI-crash restoration while the daemon remains alive. Schema-1 migration preserves a mode-0600 original backup. Other daemon sockets use separate hashed metadata files. Restoration reuses live terminal IDs; missing sessions become ended placeholders. RDP restores disconnected; Files restores without silently authenticating or refreshing. Current library identities govern profile binding and deletion leases. No password, terminal output or clipboard payload is serialized.

## Earlier milestone evidence

Published baseline v0.1.3, commit 4d5572179de259ec8f9c87ec2fa50f51b91c39ed. Existing source/build registry, Git history, historical untracked architecture document, working daemon and owner cmux sessions are preserved. Prior nine reports and all evidence remain private and intact. All new sockets, keys, buses, displays, profiles and test trees are disposable and explicitly owned. No system service, shared release controller or owner configuration was modified.

Existing build.sh and release.sh exclusively implement the build and source-release workflow. The controller owns versions, commit, tag, push and GitHub read-back. No new release adapter, manual publication or binary distribution is introduced.

| Gate | Result | Evidence |
|---|---|---|
| Independent headless engine | PASS | 18 Rust units; 11 groups/100 startup races; three layout/history/mouse groups; clean graphical dependency boundary |
| Native desktop units | PASS | 16 tests, including framed Display Control capabilities, fractional geometry, 1,000-profile migration and corruption guards |
| GTK editor and persistence | PASS fixture | Same-named distinct machines, rename/move IDs, multi-endpoint machine, cancel/duplicate/confirmed deletion, restart equality and 1,000 native sidebar entries |
| Secret Service | PASS isolated integration | Disposable private D-Bus/keyring, actual set/replace/lookup/forget; no owner keyring access |
| Embedded display control | PASS loopback | 20 real negotiated requests plus a GTK resize storm, server DeactivateAll/reactivation, decoded pixels through 2560×1440 and 3000×1800; other tabs unchanged; clipboard afterward |
| Existing RDP/GDK clipboard | PASS loopback | Three authenticated tabs; 60 Unicode transfers; 18 PNG/DIBV5 cases; five policies; reconnect; 200 switches/450 inactive offers, Files-tab clipboard isolation; 70 imports |
| Files transport and native panel | PASS loopback | Text/PNG/1 MiB binary both directions, SHA-256, unusual names, listing, overwrite, permission/key/symlink guards, cancellation, slow peer and mid-transfer loss |
| Native terminal | PASS owned compositor | 21 cards, actual Vim/less/htop and GDK; PRIMARY separately BLOCKED |
| Abrupt child | PASS bounded | Owned Bash raw-child SIGKILL, mouse reset, ECHO/ICANON, Unicode probe |
| External Windows/GNOME / physical seat | BLOCKED | No designated endpoints or approved physical input seat |
| RPM/binary distribution | BLOCKED | Source prerelease only; distribution license and package qualification pending |

External Windows/Fedora-GNOME RDP and physical GNOME PRIMARY/AltGr/IME/touchpad acceptance remain BLOCKED: RDP credentials, matching certificate identity and physical-seat qualification remain unavailable; designated hosts now include an SSH-accessible Fedora target and a Windows target. Loopback authentication, GDK and server-driven reactivation are real integration results, not external-host acceptance. The raw bitmap fixture does not qualify the GNOME/RemoteFX degradation described in [IronRDP #447](https://github.com/Devolutions/IronRDP/issues/447).

Known product limits: gateway/network fields are metadata, not a routing engine; Remmina conflicts support skip or new identity, without merge-by-ID; group default editing covers display/clipboard while other endpoint overrides remain explicit. Profiles used by connection tabs are deletion-protected until all their active tabs close; reference-counted leases release protection without an application restart. SSH terminals use OpenSSH's interactive verification/authentication; Files uses the configured key/agent or Secret Service password, and refuses unknown/changed keys. Encrypted private-key passphrase UI, rsync, recursive transfers and resume are not implemented. Portable SFTP v3 replacement preserves the old file through a backup/restore sequence but is not atomic. A lost connection can leave a named remote partial or backup; connected cancellation cleans its partial. Concurrent changes to a remote namespace are not an atomic SFTP snapshot. Native file chooser upload/download gestures and locked-keyring dialogs still need interactive desktop acceptance; transport integrity uses the production worker through an isolated driver, while panel listing/refresh uses the actual GTK window. File/HTML CLIPRDR, consumer acknowledgement for agent attachments, general termios recovery beyond Bash and daemon/host-crash persistence remain open. RPM/binary publication and product distribution licensing remain unqualified.


## Usable desktop and authorized SSH acceptance — 2026-10-11

Existing v0.1.4 source baseline and history preserved. No engine replacement, market reevaluation, shared controller edits or manually allocated versions.

`ruwac start` starts or reuses a daemon on demand. `ruwac gui` verifies the desktop executable, starts or reuses the selected daemon and opens the native application. RPC/attach now check the owned socket and same-UID peer; framed replies validate version and request correlation. An existing slow or incompatible listener is preserved and rejected. The daemon and CLI remain independent of GTK/IronRDP/libssh2 dependencies.

Normal GTK launches use one application instance and activate its window on a second launch. New terminal creates a persistent daemon session; Sessions reopens a live stable session ID without duplicating its PTY. Closing a terminal tab detaches its view. Private schema-1 workspace metadata saves terminal tab IDs/order/selection at normal window close, restores still-live sessions, and rejects corrupt, unsupported, symlink or unsafe files while preserving them. RDP/Files restoration and save-on-crash are not implemented.

Reference-counted connection leases replace the restart-only profile deletion guard. RDP owns one lease; an SSH terminal and Files tab share separate leases. A profile can be deleted after all associated tabs close. Runtime RDP display/zoom/clipboard preferences persist through the same store used by the native sidebar, tested through actual Secret Service credential lookup and authenticated loopback connection.

The existing build workflow accepts `--install-user` after successful optimized builds. Its product helper installs versioned immutable artifacts with hash/source receipts, stable commands, a desktop launcher and a previous-version pointer. It rejects command/launcher collisions, changed immutable artifacts, concurrent installation and invalid controller versions. It consumes the release version and never allocates one.
