# RUWAC implementation reconciliation

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
