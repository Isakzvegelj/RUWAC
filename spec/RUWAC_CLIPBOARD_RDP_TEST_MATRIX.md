# RUWAC desktop / protocol test matrix

## Permission-scoped native RDP lifecycle and SSH GUI transport — 2026-10-11

Compiled package 0.1.15 exposes read-only `rdp list/query`, saved-profile `rdp open`, and explicit-view `rdp reconnect/disconnect` through the existing authenticated native GUI actor. RDP control requires separate owner consent, disabled by default; permission revocation applies at dispatch. Activation additionally requires the existing focus permission. Background open/reconnect preserves the selected tab, neighboring transports and desktop clipboard ownership. Credentials resolve inside the existing Secret Service worker, never in CLI arguments or API results. The response acknowledges connection startup; callers query phase and decoded updates to establish actual readiness.

Typed stable endpoint/view/tab targets, exact parameter validation, a 64-RDP-view cap, stale/detached target rejection and explicit refusal of idempotency keys bound the operation. Metadata contains phase, framebuffer dimensions, transport identity, decoded-update count and broker ownership; it excludes credentials, certificate paths, pixels and private error text. Audit records contain fixed operations, valid typed identifiers and verdicts without arbitrary payloads. Disconnect leaves the view available for reconnection; closing removes it and makes its identifier stale.

The headless CLI uses the same typed request over verified SSH with `--host`, bounded framed replies and existing trusted same-UID GUI sockets. Neither a missing remote GUI nor a failed tunnel launches a GUI/daemon or falls back to the headless endpoint. SSH authentication and endpoint trust retain existing checks.

Seven actual authenticated disposable local RDP groups pass, including generated credentials resolved by the private Secret Service, CredSSP/framebuffer updates, background disconnect/reconnect, independent focus consent, runtime revocation, preserved neighboring connections/GDK clipboard, rejected credential/unknown parameters, payload-free auditing and stale views. Three actual SSH groups pass through EVF and an owned Unix socket forward to a locally rendered CVF GTK fixture. This proves remote CLI transport and permissions; it does not prove graphical rendering on EVF or external RDP login. Seven library, existing native workspace/focus/recovery, headless PTY/history and GTK regressions repeat on the final stamped artifacts. PRIMARY remains blocked on the owned compositor.

CLI SHA-256: `58f05b7692d335c6c0eb5bb557ecde2b61657e26135709cc6c9a3c6c2f0ddd61`. GTK SHA-256: `6bdcdff7053ccf1f584617f2c80060c90af08731c3ee54c8a79ace138df7d767`. Build: `build.sh --release --test --desktop`; publication: existing `release.sh beta`, with versioning owned by the controller. Stable qualification remains open for external Windows/Fedora login/clipboard, physical GNOME input, full automation/agent/file scope and qualified Fedora binary packaging. Host inventory, workspace open/restore, GUI event subscriptions and RDP input automation are not implemented by this increment.


## Machine library paging and reviewed merge — 2026-10-11

Compiled package 0.1.15 adds accessible previous/next controls and a 50-profile page limit. A 1,000-profile native fixture reaches every endpoint across 20 pages; search/filter changes reset the page and refresh clamps it. Empty group labels are bounded separately; group editing remains available. Endpoint IDs, native connect/edit/favorite handlers and complete search remain intact.

Remmina metadata can now merge only into an explicitly selected existing endpoint. The owner reviews a native field-by-field dialog before Apply. Imported name, full group, protocol, server, username and tags change; endpoint/machine identity, credential reference, trust/key paths, preferences, favorites, recency and inheritance remain local. Original Remmina files and excluded passwords/key paths are untouched. The UI warns to verify changed destinations before connecting. Active endpoints are protected, and Apply rechecks both identity/content and active usage after review. Cancel writes nothing, failed persistence restores in-memory metadata, and existing Undo restores the previous private library. Changing the selected source path invalidates the preview.

Seven actual GTK fixture groups pass: 1,000-profile paging/search, explicit target and native review/cancel, persistence and one-endpoint-only changes at the library cap, Undo, stale review, active endpoint rejection and restart. Actions are synthetic widget/dialog callbacks on an owned compositor; no physical input or external login is inferred. Twenty-one GTK unit tests pass, including policy/identity merge preservation. Fourteen authenticated local RDP regression groups pass on the pre-stamp candidate, including the existing 70-profile import check updated to assert the visible page cap, real text/PNG clipboard ownership and resizing. Seven RDP lifecycle/editor/display groups are repeated on the final stamped build. Headless canonical copy, nesting, immutable history, daemon recovery and the existing GTK regressions are repeated by the normal stamped workflow.

CLI SHA-256: `58f05b7692d335c6c0eb5bb557ecde2b61657e26135709cc6c9a3c6c2f0ddd61`. GTK SHA-256: `6bdcdff7053ccf1f584617f2c80060c90af08731c3ee54c8a79ace138df7d767`. Compile/test: `build.sh --release --test --desktop`. Publication: existing `release.sh beta`; controller owns versioning. Builds and raw receipts remain in the build workspace. Source publication includes no machine-specific paths or agent instructions.

This advances F-01/F-02; it does not complete inherited network route execution, external RDP/physical GNOME acceptance, agent/file/automation milestones or Fedora binary/RPM qualification. Full product completion remains unproven.


## Headless Screen and immutable copy increment — 2026-10-11

Compiled package 0.1.15. The existing daemon, authenticated protocol, GTK frontend and RDP engine remain the foundation. Headless copy now uses canonical immutable history pins, Unicode grapheme/cell selection, rectangular and soft-wrap copying, search, word/page navigation and non-destructive cancellation. The terminal adds a window chooser, command prompt, help, zoom and configurable Control prefixes/key mappings. All modal mouse/paste input is contained; original terminal modes are restored. Configuration errors are rejected before raw mode.

The normal build workflow includes nine real-PTY acceptance groups: Unicode internal copy/paste, rectangle and physical wrap, oversize/cancel preservation, chooser/prompt/help/zoom, prefix configuration and actual nested RUWAC layers two through five. This is RUWAC nesting evidence; it does not claim GNU Screen/tmux combinations or physical keyboard qualification. Three authenticated API groups prove frozen output across live output and actual resize, owner/pane binding, idempotency-cache bypass refusal, eight-pin global cap, one pin per connection, unchanged terminal revision, disconnect reclamation and actual 30-second expiry. A terminal unit verifies immutable Unicode rows across feed/resize and the 50,000-row tail bound. The first rectangle fixture mistakenly searched common text; the corrected unique search passes, and the failed receipt remains private.

Limits: 50,000 physical rows or 8 MiB per pin, 128 KiB per row/page text, eight global pins, one per connection, 30-second idle expiry; removed tail rows are explicitly reported. Internal selection remains capped at 256 KiB and preserves the prior buffer on failure. Snapshots never enter recovery checkpoints. GTK history reads use pins when supported and preserve older-daemon fallback.

The stamped native build repeats actual older-daemon compatibility, two native recovery groups, eight workspace groups, nine GUI API groups, 21 GTK/GDK terminal cards and seven authenticated isolated RDP/editor/display groups. PRIMARY is unavailable on the owned compositor. Earlier full 14-group loopback RDP text/image/ownership evidence remains historical; this increment makes no external Windows/Fedora login claim. Local compilation uses `build.sh --release --test --desktop`; publication uses the existing `release.sh beta`, whose controller owns the version. GitHub currently publishes source, with no binary compilation or RPM asset qualification.

CLI SHA-256: `58f05b7692d335c6c0eb5bb557ecde2b61657e26135709cc6c9a3c6c2f0ddd61`. GTK SHA-256: `6bdcdff7053ccf1f584617f2c80060c90af08731c3ee54c8a79ace138df7d767`. Full stable qualification remains open: external RDP, physical GNOME/PRIMARY/HiDPI/accessibility, mixed-multiplexer nesting, agent/file/automation scope and qualified Fedora packaging. Owner sessions and configuration are preserved.


## Prior RDP presentation and Screen correction — 2026-10-11

Toolbar reconnect/disconnect uses the same authenticated IronRDP and FocusBroker paths. Qualification uses disposable credential-backed Secret Service profiles, real TLS/CredSSP sessions, decoded frames and GDK clipboard consumers. External Windows/Fedora and physical-seat acceptance stay separate.



## Validation for prior RDP increment — 2026-10-11

Compiled package 0.1.12: the normal build/release workflow runs standard Rust/headless regressions, including 11 groups and 100 startup races, seven recovery groups and two real-PTY previous/literal/checkpoint-migration groups. Authorized SSH repeated both previous-window groups against a private endpoint candidate; no live session was replaced. The final native build also passes an actual older-daemon compatibility case: unsupported previous-window capability is visible and subsequent input reaches the same child, without freezing navigation.

The stamped candidate passes two native recovery groups, eight workspace groups, nine GUI API groups and 21 native terminal/GDK cards; PRIMARY is explicitly blocked on the owned compositor. Fourteen authenticated local RDP groups pass in the full presentation regression, including 60 text transfers, 18 PNG/DIBV5 cases, 200 tab switches/450 inactive offers, 20 negotiated resizes plus the GTK resize storm, editor/keyring and 1,000-profile checks. Six editor/display/RDP groups are repeated against the final stamped build. Actual visible toolbar callbacks reconnect using a disposable Secret Service credential and verified CA, decode a new framebuffer and preserve both neighboring transports; zoom/fit and unknown-action behavior pass. The actual GTK render is retained privately, with generated server pixels distinguished from external Windows login.

CLI SHA-256: `129d2d784653fd67238ca93cef11eb858c690f530a2d43f1f89687aa98e4ac07`. GTK SHA-256: `9515d9645262c6badcf00d484ee4386efc629d0b699e82d85a1755c277fcb9b2`. Builds and raw fixtures stay in the build workspace. Original source is Apache-2.0. Standard theme icons are resolved from the native desktop; no upstream artwork or code was copied. Earlier failed compilation receipts are preserved, including a fixture JSON GString conversion corrected before publication.

Stable is still unqualified: authorized Windows/Fedora login and clipboard/display/input, physical GNOME/PRIMARY/HiDPI, full headless selection/chooser/nesting, agent/file/automation milestones and Fedora package distribution remain open. `build.sh --release --package rpm` returns 78 with “RPM candidate packaging is blocked: no qualified app-linux package adapter installed.” Exact private feedback is recorded for 0rig; the node relay remains unavailable. Shared release infrastructure and versions were not edited. GitHub publishes source/tag pages; local build.sh compiles and the private installer deploys binaries.


## Current private daemon checkpoint and ended-history recovery — 2026-10-11

Internal clipboard registers are not serialized into daemon checkpoints. Native recovery can copy ended history into the independent internal buffer. Four genuinely authenticated isolated IronRDP clipboard/mixed-workspace groups remain passing; external-host and physical PRIMARY acceptance remain separate.


## Current persistent graphical identities and permission-scoped workspace API — 2026-10-11

Native window activation now gates remote clipboard ownership, including IPC focus changes in the background. Four authenticated isolated RDP groups pass, including three live sessions against an independent foreground GDK clipboard sentinel, mixed transport retention and Files isolation. External interoperability, PRIMARY and physical desktop remain separate gates.


## Current native shortcuts, appearance and safe diagnostics — 2026-10-11

Window-level Alt-number tab changes retain the existing focus/clipboard ownership transitions. The authenticated isolated RDP clipboard regression is repeated separately from the synthetic shortcut RDP view. External Windows/Fedora and physical desktop clipboard remain unqualified.


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
