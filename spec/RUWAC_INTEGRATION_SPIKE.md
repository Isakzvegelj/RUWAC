# RUWAC implementation and qualification evidence

Existing v0.1.3 foundation preserved; 2026-10-11 workspaces/display/files increment.

Native machine/endpoint dialogs create, edit, duplicate, move and confirm deletion of metadata, with stable machine and endpoint identities, shared multi-endpoint machines, full nested group paths, endpoint labels, tags, favorites and recents. Groups have stable IDs and explicit inherited display/clipboard defaults; saved endpoint overrides are retained. Schema 1 migrates the legacy array atomically and keeps a private previous-library backup for Undo. Invalid/unknown schemas, duplicate IDs, unsafe/special files and oversized libraries fail closed. Metadata remains 0700/0600 and contains credential references, never secret values.

Set/replace/forget credential actions use system Secret Service through secret-tool and GIO/libsecret off the GTK thread. Passwords travel through stdin, never process arguments; reads and operations have caps/deadlines, and unavailable/locked providers produce recoverable messages. Normal RDP retrieves credentials only after explicit connect; Files authenticates after strict known-host verification. No owner secret store was inspected or changed.

Embedded RDP offers fit, native physical pixels, explicit stretch, zoom, fullscreen, scrolling/panning, remote wheel routing and Ctrl-wheel zoom. Drawing and input share the per-tab geometry; native mode accounts for GDK surface scaling. Dynamic mode requests a legal single-monitor layout after 250 ms debounce, suppresses duplicate geometry requests during reactivation and bounds SVC/DVC fragments before reassembly and negotiated capabilities/area checks, with a visible fixed-resolution fallback. Deactivate All runs IronRDP's activation factory in the same connection/tab with bounded cancellation, fresh decoder/framebuffer, held-input release, stale clipboard invalidation and share-ID updates. TLS/authentication failures remain nonretryable; reconnect preference is applied. Changes to display, zoom, fullscreen and clipboard policy debounce into the same endpoint metadata store; applying group defaults preserves explicit trust/key/reconnect settings.

The pinned ironrdp-displaycontrol 0.8 client decodes bare capability fields while its server sends the MS-RDPEDISP header. A bounded 20-byte adapter validates the complete PDU, removes that header and delegates parsing/monitor encoding to IronRDP. The regression proves a 3840×2400 area rather than the misinterpreted header fields. No upstream or shared release infrastructure was modified.

SSH opens a daemon-owned OpenSSH PTY plus a native Files tab. SFTP uses ssh2 0.9.6/libssh2, verified known_hosts, validated argv for the terminal and protocol paths for Files. Native file dialogs choose local sources/destinations; the panel lists permissions, supports navigation, refresh, explicit overwrite or renamed/skip destinations, byte progress and cancellation. Transfers use 64 KiB buffers, a 32 GiB cap, exclusive 0600 partials and SHA-256 checks. Symlink traversal/overwrite and parent traversal are rejected. File errors stay in the panel and do not terminate terminal or RDP tabs.

## Executed acceptance

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

Observed loopback resize completion: 84.3–298.2 ms, mean 194.3 ms across 20 requests. Measurements include authenticated transport, server reactivation, client decoding and fixture polling, not physical photon latency. Native 1,000-profile load/ready was 533.7 ms; query/GTK fixture roundtrip 217.1 ms. Pure 1,000-profile migration/search/serialization measured 25 ms in the focused unit run. Complete-update intervals and per-transition GTK/server RSS/CPU samples are in sanitized acceptance metadata. These are owned-fixture observations on a busy VM, not idle-qualified performance guarantees. Historical matched CLI benchmarks are retained as prior R4 evidence; the headless source is unchanged.

## Reproduction

```sh
./build.sh --release --test --desktop
cargo build --locked --release --manifest-path ruwac/desktop/Cargo.toml --examples
python3 ruwac/tests/integration/rdp_clipboard.py --binary "$CARGO_TARGET_DIR/release/ruwac" --gui "$CARGO_TARGET_DIR/release/ruwac-desktop" --server "$CARGO_TARGET_DIR/release/examples/rdp-fixture" --evidence "$RUWAC_EVIDENCE_DIR"
python3 ruwac/tests/integration/sftp.py --driver "$CARGO_TARGET_DIR/release/examples/files-fixture" --binary "$CARGO_TARGET_DIR/release/ruwac" --gui "$CARGO_TARGET_DIR/release/ruwac-desktop" --evidence "$RUWAC_EVIDENCE_DIR"
python3 ruwac/tests/integration/native_rust.py --binary "$CARGO_TARGET_DIR/release/ruwac" --gui "$CARGO_TARGET_DIR/release/ruwac-desktop" --evidence "$RUWAC_EVIDENCE_DIR" --primary-unavailable
python3 ruwac/tests/integration/abrupt_child.py --binary "$CARGO_TARGET_DIR/release/ruwac" --evidence "$RUWAC_EVIDENCE_DIR"
"$CARGO_TARGET_DIR/release/examples/files-fixture" credentials
./release.sh beta
```

Native tests require an owned Weston/Xvfb seat, private D-Bus, GDK Wayland, the fixture X display in RUWAC_XTEST_DISPLAY, Python GI/Pillow and OpenSSL. The credential driver must run only with a disposable unlocked Secret Service provider and private HOME/data directories. SFTP uses a current-user loopback sshd with generated host/client keys and its own config; no system sshd/PAM/accounts/firewall changes. The release controller allocates version/commit/tag/publication; the commands above do not manually publish.

Pre-publication candidate executable hashes (version stamping rebuilds them; final hashes are retained in the publication receipt):

| Artifact | SHA-256 |
|---|---|
| ruwac | 5de3f6d75805acfb761b14615aba14afa77ad5b613184726ecbaa46f794a95d7 |
| ruwac-desktop | 59d51441c743e60380a82b8f41b4e2fe8b9a0944e26156c97dc001cd8c0119d0 |
| files-fixture | 409a6c3bda8c79a9f7b9b49d0f0d77dbe9bcd5ecbeedad70e44390c23ac17027 |
| rdp-fixture | 573f71a12e722532b556548a49ccbc85bb2fac3c8c1787547f85c39d7fb1fc2a |

Private evidence is retained under the registered build authority in the workspaces-display-files-r5 capsule: prior-reports, build-r5-final.log, profile-benchmark.log, rdp-r5-final.log, rdp-native-acceptance.json and per-run receipts, sftp-r5-final.log/sftp-acceptance.json, isolated keyring receipt, terminal regression, abrupt-child receipt, candidate hashes, dependency metadata, exact commands and publication verification. Failed trials remain retained, including the delete borrow panic, initial overwrite/listing defects, stale-binary trials and mismatched lossy-pixel assertions. Public metadata excludes private paths, generated credentials, raw payloads and logs.

## Remaining defects / qualification

External Windows/Fedora-GNOME RDP and physical GNOME PRIMARY/AltGr/IME/touchpad acceptance remain BLOCKED: RDP credentials, matching certificate identity and physical-seat qualification remain unavailable; designated hosts now include an SSH-accessible Fedora target and a Windows target. Loopback authentication, GDK and server-driven reactivation are real integration results, not external-host acceptance. The raw bitmap fixture does not qualify the GNOME/RemoteFX degradation described in [IronRDP #447](https://github.com/Devolutions/IronRDP/issues/447).

Known product limits: gateway/network fields are metadata, not a routing engine; Remmina conflicts support skip or new identity, without merge-by-ID; group default editing covers display/clipboard while other endpoint overrides remain explicit. Profiles used by connection tabs are deletion-protected until all their active tabs close; reference-counted leases release protection without an application restart. SSH terminals use OpenSSH's interactive verification/authentication; Files uses the configured key/agent or Secret Service password, and refuses unknown/changed keys. Encrypted private-key passphrase UI, rsync, recursive transfers and resume are not implemented. Portable SFTP v3 replacement preserves the old file through a backup/restore sequence but is not atomic. A lost connection can leave a named remote partial or backup; connected cancellation cleans its partial. Concurrent changes to a remote namespace are not an atomic SFTP snapshot. Native file chooser upload/download gestures and locked-keyring dialogs still need interactive desktop acceptance; transport integrity uses the production worker through an isolated driver, while panel listing/refresh uses the actual GTK window. File/HTML CLIPRDR, consumer acknowledgement for agent attachments, general termios recovery beyond Bash and daemon/host-crash persistence remain open. RPM/binary publication and product distribution licensing remain unqualified.


## Usable desktop and authorized SSH acceptance — 2026-10-11

`ruwac start` starts or reuses a daemon on demand. `ruwac gui` verifies the desktop executable, starts or reuses the selected daemon and opens the native application. RPC/attach now check the owned socket and same-UID peer; framed replies validate version and request correlation. An existing slow or incompatible listener is preserved and rejected. The daemon and CLI remain independent of GTK/IronRDP/libssh2 dependencies.

Normal GTK launches use one application instance and activate its window on a second launch. New terminal creates a persistent daemon session; Sessions reopens a live stable session ID without duplicating its PTY. Closing a terminal tab detaches its view. Private schema-1 workspace metadata saves terminal tab IDs/order/selection at normal window close, restores still-live sessions, and rejects corrupt, unsupported, symlink or unsafe files while preserving them. RDP/Files restoration and save-on-crash are not implemented.

Reference-counted connection leases replace the restart-only profile deletion guard. RDP owns one lease; an SSH terminal and Files tab share separate leases. A profile can be deleted after all associated tabs close. Runtime RDP display/zoom/clipboard preferences persist through the same store used by the native sidebar, tested through actual Secret Service credential lookup and authenticated loopback connection.

The existing build workflow accepts `--install-user` after successful optimized builds. Its product helper installs versioned immutable artifacts with hash/source receipts, stable commands, a desktop launcher and a previous-version pointer. It rejects command/launcher collisions, changed immutable artifacts, concurrent installation and invalid controller versions. It consumes the release version and never allocates one.

CVF: 18 headless Rust units; 11 headless groups including 100 startup races; three desktop-state groups; 16 desktop Rust units; four startup/security groups; three native workspace groups; a normal non-fixture single-instance/crash smoke; 21 native terminal cards (PRIMARY remains separately blocked); all 12 authenticated loopback RDP groups; six SFTP groups; seven installer cases. The extended RDP suite uses its own private HOME, D-Bus and disposable Secret Service provider. Failed selector/provider trials remain private evidence.

Authorized Fedora host: the exact candidate CLI and desktop binaries run without missing dynamic libraries. All 11 headless groups/100 startup races, four startup/security groups and three state groups pass there. Three actual SSH remote-PTY attach/type/detach/reattach cycles preserve the same child PID, deliver exact Unicode bytes and negotiate a 105-column, 30-content-row pane inside a 31-row SSH PTY. This is real SSH/TUI acceptance; it is not RDP or physical-GNOME acceptance.

The matched CLI benchmark uses 30 alternating samples per action at 1/5/20/40 panes. All 12 CLI median ratios are within 2× tmux. VM utilization before the run was 24.7%, so the run is not idle-qualified. Key-to-emitted-render median/p95: 37.05/39.81 ms; observer-based, not physical latency. At 40 panes daemon/TUI RSS: 13.96/5.23 MiB, excluding child interpreters.

```sh
./build.sh --release --test --desktop
cargo build --locked --release --manifest-path ruwac/desktop/Cargo.toml --examples
python3 ruwac/tests/integration/startup.py --binary "$CARGO_TARGET_DIR/release/ruwac" --evidence "$RUWAC_EVIDENCE_DIR"
python3 ruwac/tests/integration/workspace.py --binary "$CARGO_TARGET_DIR/release/ruwac" --gui "$CARGO_TARGET_DIR/release/ruwac-desktop" --evidence "$RUWAC_EVIDENCE_DIR"
python3 ruwac/tests/integration/rdp_clipboard.py --binary "$CARGO_TARGET_DIR/release/ruwac" --gui "$CARGO_TARGET_DIR/release/ruwac-desktop" --server "$CARGO_TARGET_DIR/release/examples/rdp-fixture" --evidence "$RUWAC_EVIDENCE_DIR"
python3 ruwac/tests/integration/sftp.py --driver "$CARGO_TARGET_DIR/release/examples/files-fixture" --binary "$CARGO_TARGET_DIR/release/ruwac" --gui "$CARGO_TARGET_DIR/release/ruwac-desktop" --evidence "$RUWAC_EVIDENCE_DIR"
python3 ruwac/tests/integration/ssh_terminal.py --host "$RUWAC_AUTHORIZED_SSH_HOST" --binary "$RUWAC_REMOTE_BINARY" --receiver "$RUWAC_REMOTE_RECEIVER" --remote-evidence "$RUWAC_REMOTE_EVIDENCE" --evidence "$RUWAC_EVIDENCE_DIR"
./release.sh beta
./release.sh --verify
./build.sh --release --desktop --install-user
ruwac start
ruwac gui
```
Native acceptance requires an owned compositor and test D-Bus environment. SSH acceptance requires an authorized endpoint, existing key authentication and a verified host key. Exact node commands, hashes, retained failed trials and raw receipts are private in the usability-r6 capsule. Release allocation/publication remains the existing controller's responsibility.
