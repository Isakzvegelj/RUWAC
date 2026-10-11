# RUWAC desktop / protocol test matrix

## Current authenticated host CLI and discovery — 2026-10-11

No graphical clipboard/RDP transport changes. Retain the preceding native compositor and authenticated loopback evidence; SSH input acceptance does not establish external RDP login, agent attachment or physical input.


## Current individual view management — 2026-10-11

**Views…** opens a native dialog for the selected mixed workspace. Each leaf offers **Move to own tab** and **Close view**. Removing a leaf collapses its divider and keeps the sibling widget/tree alive. Moving preserves the underlying connection; closing a terminal detaches its presentation while daemon programs keep running, closing RDP disconnects only that connection, and closing Files cancels its work and releases its own lease. Closing the last leaf removes the empty tab. Dialog errors remain visible if a view disappeared. The dialog takes local focus before opening; remote clipboard ownership cannot follow it.

Connection registries remove only the selected leaf. Profile deletion protection survives a move and remains until the last associated view closes. Workspace schema 2 records the resulting topology without a schema change; no daemon protocol, engine or dependency replacement was introduced. Existing terminal splits inside a session remain independent of mixed GUI leaves.

Eight native workspace groups pass: live same-PID reopening, migration/backup, profile leases, mixed GUI-crash restoration, separate daemon sockets, unsafe metadata, individual view management and the empty-workspace lifecycle. Actual dialog buttons close a nested terminal leaf, preserve and reopen its daemon PID, move one leaf into a tab and close a sole-view tab. Closing all presentations checkpoints zero tabs while original daemon PIDs survive; New terminal creates a usable view from that empty workspace.

The focused RDP suite passes three groups against actual TLS-verified/CredSSP loopback sessions and real GTK/GDK. A moved RDP leaf retains its transport ID. Closing the selected RDP disconnects only that view; both neighboring transports remain unchanged. A replacement connection authenticates, decodes frames and transfers CLIPRDR text. Existing nested-tree and inactive-offer ownership checks also pass. The full 13-group clipboard/display/editor acceptance remains retained as preceding-batch evidence; it was not rerun in full for this view-management change.

All six real isolated SSH/SFTP groups pass. The native panel moves out of a terminal/Files split, refreshes its real directory listing successfully, and closes while preserving its neighboring terminal PID. SHA-256 integrity, cancellation, slow peer and transport-loss regression groups pass. An in-flight transfer during GTK reparenting is not separately qualified. All 21 native terminal cards pass, with PRIMARY explicitly unavailable on the owned compositor. Failed test assumptions about restored tab kinds, session chooser opening and reordered RDP tabs are retained privately and corrected by locating the actual resource.


## Current mixed workspace increment — 2026-10-11

The owned nested Wayland compositor exercises real GTK/GDK. Six workspace cases cover live same-PID reopening, schema-1 migration with private backup, profile leases during reparenting, nested terminal/RDP/Files restoration after GUI SIGKILL, separate socket metadata and unsafe symlink preservation. The restored RDP metadata test uses a disconnected placeholder; it does not establish remote login.

All 13 full RDP groups pass against disposable authenticated IronRDP servers, including the new live mixed-tree test. Three authenticated transports keep their connection IDs during join/separate; actual CLIPRDR text and inactive offers are verified while mixed. After compacting controls, six editor/keyring/display/Files groups and three authenticated mixed/ownership groups are rerun. The isolated Secret Service provider performs real credential lookup for normal sidebar connections. These are real local TLS/CredSSP, framebuffer and clipboard integrations, not external-host acceptance.

Six SFTP groups pass with real isolated SSH transport, SHA-256 integrity, cancellation, slow peer/loss and native panel listing. Native terminal acceptance retains 21 passing cards, actual Vim/less/htop, independent GDK clipboard consumers and actual toolbar PNG staging followed by GUI-crash cleanup; PRIMARY is separately unavailable on this compositor. Reparenting an active file operation is preserved by implementation, but not yet separately qualified during an in-flight transfer.

Matched API benchmarks use 30 alternating samples for each action at 1/5/20/40 panes. All 12 median ratios are within 2× tmux; the VM was 25.7% busy, so idle-qualified acceptance remains open. Key-to-emitted-render median/p95 is 36.16/38.12 ms, with a 5 ms observer; this is not physical display latency. At 40 panes daemon/TUI RSS is 13.82/5.22 MiB, excluding child interpreters. The independent daemon/CLI source is preserved.

Remaining product limits: individual mixed-pane close/move UI, stable GUI view IDs and exact per-leaf focus restoration; durable terminal history after daemon loss; permission-scoped GUI automation and host CLI; supported local/remote agent image consumption; encrypted SSH-key passphrase UI, recursive/resumable/rsync files; physical GNOME PRIMARY/AltGr/IME/touchpad/accessibility; external Windows/Fedora RDP login/clipboard and matched Remmina/FreeRDP performance; qualified RPM/binary publication and broader governance/CI. Authorized external endpoints are identified, but credentials must be entered through the owner’s profile editor and certificate identity must validate. No owner credential store was used in tests. Apache-2.0 is now approved for original source; dependency notices and packaging qualification remain separate.

## Earlier milestone evidence

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

Optional owner-operated acceptance: on an explicitly approved disposable seat, use artificial Unicode sentinels and PNGs, three authorized RDP accounts and a terminal. Verify inactive tabs never replace CLIPBOARD, exercise the five policies, copy/search/Screen navigation, PRIMARY, AltGr/IME, touchpad, send-key fallbacks and fractional scaling. External-host testing needs a designated OS/host, account reference, trusted certificate/CA and permitted-use scope; no discovery or credential harvesting.


## Usable desktop and authorized SSH acceptance — 2026-10-11

CVF: 18 headless Rust units; 11 headless groups including 100 startup races; three desktop-state groups; 16 desktop Rust units; four startup/security groups; three native workspace groups; a normal non-fixture single-instance/crash smoke; 21 native terminal cards (PRIMARY remains separately blocked); all 12 authenticated loopback RDP groups; six SFTP groups; seven installer cases. The extended RDP suite uses its own private HOME, D-Bus and disposable Secret Service provider. Failed selector/provider trials remain private evidence.

Authorized Fedora host: the exact candidate CLI and desktop binaries run without missing dynamic libraries. All 11 headless groups/100 startup races, four startup/security groups and three state groups pass there. Three actual SSH remote-PTY attach/type/detach/reattach cycles preserve the same child PID, deliver exact Unicode bytes and negotiate a 105-column, 30-content-row pane inside a 31-row SSH PTY. This is real SSH/TUI acceptance; it is not RDP or physical-GNOME acceptance.

The matched CLI benchmark uses 30 alternating samples per action at 1/5/20/40 panes. All 12 CLI median ratios are within 2× tmux. VM utilization before the run was 24.7%, so the run is not idle-qualified. Key-to-emitted-render median/p95: 37.05/39.81 ms; observer-based, not physical latency. At 40 panes daemon/TUI RSS: 13.96/5.23 MiB, excluding child interpreters.

External Windows/Fedora GNOME RDP login and clipboard interoperability are unverified. The authorized Fedora host is reachable using an existing SSH key and advertises RDP port 3389; its configured certificate has CN=GNOME and no subjectAltName, so hostname validation needs a supported trust solution. No password has been received. Physical PRIMARY/AltGr/IME/touchpad qualification, mixed terminal/RDP/Files splits, complete workspace/crash restoration, agent image consumption, encrypted SSH key passphrases, rsync/resume/recursive transfers and RPM/binary distribution remain open. The owner source-license decision is pending.
