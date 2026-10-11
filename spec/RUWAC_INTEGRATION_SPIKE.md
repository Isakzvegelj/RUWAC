# RUWAC implementation and qualification evidence

## Headless Screen and immutable copy increment — 2026-10-11

Compiled package 0.1.13. The existing daemon, authenticated protocol, GTK frontend and RDP engine remain the foundation. Headless copy now uses canonical immutable history pins, Unicode grapheme/cell selection, rectangular and soft-wrap copying, search, word/page navigation and non-destructive cancellation. The terminal adds a window chooser, command prompt, help, zoom and configurable Control prefixes/key mappings. All modal mouse/paste input is contained; original terminal modes are restored. Configuration errors are rejected before raw mode.

The normal build workflow includes nine real-PTY acceptance groups: Unicode internal copy/paste, rectangle and physical wrap, oversize/cancel preservation, chooser/prompt/help/zoom, prefix configuration and actual nested RUWAC layers two through five. This is RUWAC nesting evidence; it does not claim GNU Screen/tmux combinations or physical keyboard qualification. Three authenticated API groups prove frozen output across live output and actual resize, owner/pane binding, idempotency-cache bypass refusal, eight-pin global cap, one pin per connection, unchanged terminal revision, disconnect reclamation and actual 30-second expiry. A terminal unit verifies immutable Unicode rows across feed/resize and the 50,000-row tail bound. The first rectangle fixture mistakenly searched common text; the corrected unique search passes, and the failed receipt remains private.

Limits: 50,000 physical rows or 8 MiB per pin, 128 KiB per row/page text, eight global pins, one per connection, 30-second idle expiry; removed tail rows are explicitly reported. Internal selection remains capped at 256 KiB and preserves the prior buffer on failure. Snapshots never enter recovery checkpoints. GTK history reads use pins when supported and preserve older-daemon fallback.

The stamped native build repeats actual older-daemon compatibility, two native recovery groups, eight workspace groups, nine GUI API groups, 21 GTK/GDK terminal cards and six authenticated isolated RDP/editor/display groups. PRIMARY is unavailable on the owned compositor. Earlier full 14-group loopback RDP text/image/ownership evidence remains historical; this increment makes no external Windows/Fedora login claim. Local compilation uses `build.sh --release --test --desktop`; publication uses the existing `release.sh beta`, whose controller owns the version. GitHub currently publishes source, with no binary compilation or RPM asset qualification.

CLI SHA-256: `4cf70c7fb61dab608101ca83fb55bb5685c6fbb08130a5d968bbf569861d8fd5`. GTK SHA-256: `6371ea14c3899717f00bc518e246c26b0ef2004373fd9f6ccd7f5656be7adbb9`. Full stable qualification remains open: external RDP, physical GNOME/PRIMARY/HiDPI/accessibility, mixed-multiplexer nesting, agent/file/automation scope and qualified Fedora packaging. Owner sessions and configuration are preserved.


## Prior RDP presentation and Screen correction — 2026-10-11

Existing compiled native presentation was extended in place. Tests use actual GTK controls and authenticated disposable RDP endpoints; screenshots render the compiled window. Source-only GitHub publication does not supply a binary build. No owner keyring/clipboard or running terminal is used as a fixture.



## Validation for prior RDP increment — 2026-10-11

Compiled package 0.1.12: the normal build/release workflow runs standard Rust/headless regressions, including 11 groups and 100 startup races, seven recovery groups and two real-PTY previous/literal/checkpoint-migration groups. Authorized SSH repeated both previous-window groups against a private endpoint candidate; no live session was replaced. The final native build also passes an actual older-daemon compatibility case: unsupported previous-window capability is visible and subsequent input reaches the same child, without freezing navigation.

The stamped candidate passes two native recovery groups, eight workspace groups, nine GUI API groups and 21 native terminal/GDK cards; PRIMARY is explicitly blocked on the owned compositor. Fourteen authenticated local RDP groups pass in the full presentation regression, including 60 text transfers, 18 PNG/DIBV5 cases, 200 tab switches/450 inactive offers, 20 negotiated resizes plus the GTK resize storm, editor/keyring and 1,000-profile checks. Six editor/display/RDP groups are repeated against the final stamped build. Actual visible toolbar callbacks reconnect using a disposable Secret Service credential and verified CA, decode a new framebuffer and preserve both neighboring transports; zoom/fit and unknown-action behavior pass. The actual GTK render is retained privately, with generated server pixels distinguished from external Windows login.

CLI SHA-256: `129d2d784653fd67238ca93cef11eb858c690f530a2d43f1f89687aa98e4ac07`. GTK SHA-256: `9515d9645262c6badcf00d484ee4386efc629d0b699e82d85a1755c277fcb9b2`. Builds and raw fixtures stay in the build workspace. Original source is Apache-2.0. Standard theme icons are resolved from the native desktop; no upstream artwork or code was copied. Earlier failed compilation receipts are preserved, including a fixture JSON GString conversion corrected before publication.

Stable is still unqualified: authorized Windows/Fedora login and clipboard/display/input, physical GNOME/PRIMARY/HiDPI, full headless selection/chooser/nesting, agent/file/automation milestones and Fedora package distribution remain open. `build.sh --release --package rpm` returns 78 with “RPM candidate packaging is blocked: no qualified app-linux package adapter installed.” Exact private feedback is recorded for 0rig; the node relay remains unavailable. Shared release infrastructure and versions were not edited. GitHub publishes source/tag pages; local build.sh compiles and the private installer deploys binaries.


## Current private daemon checkpoint and ended-history recovery — 2026-10-11

The retained headless Rust engine now writes private schema-one checkpoints containing the existing session/window/pane IDs, names, shared split tree, focus, zoom, canonical Unicode/tab/wrapped history, visible cell styles and known child exit codes. Restored panes are read-only historical resources: ended=true, restored=true and pid=null. No shell or child command is rerun. New terminals can be created alongside recovered history; closed resources disappear from the next checkpoint. The CLI/TUI, native session chooser and desktop status distinguish restored history from live processes.

The ordinary daemon stores terminal-history.json in the private RUWAC directory beneath XDG_STATE_HOME (or the standard home state directory), outside the volatile runtime tree. Custom sockets use a history.json sibling unless RUWAC_STATE_FILE explicitly supplies an existing private durable parent. Checkpoint files and locks are owned, regular, singly linked and mode 0600; parents are owned, nonsymlink and 0700. A separate checkpoint lock prevents two socket targets from owning one archive. Startup rejects and preserves corrupt, unknown-schema, oversized or unsafe state. Loaded graphs validate IDs, membership, layouts, active targets and canonical cell/history shapes before serving restored resources.

Changed state checkpoints on a two-second timer. Disk serialization is buffered in a bounded background worker; monotonic revision ordering prevents an older periodic write from replacing a newer explicit save. Atomic rename and file/directory fsync protect completed checkpoints. The daemon.checkpoint RPC waits for a completed write; normal daemon.stop checkpoints before shutdown. Read-only idle queries do not repeatedly rewrite state. health.persistence reports the last completed revision, pending work and write failure; a failed write preserves the previous completed archive and can be retried after repair of the targeted fixture/configuration.

Each pane retains up to 50,000 canonical history rows or 512 KiB of text; dropped_lines is explicit when truncation occurs. The overall archive has a 128 MiB streaming/read cap. Cell snapshots retain Unicode, wide-cell flags and colors; ended panes disable mouse, bracketed-paste and application-input modes. Internal clipboard registers, raw input, RPC/idempotency logs, argv, client identities and credentials are excluded. This is a private terminal-state file, not an ordinary output/request log; already displayed text may contain sensitive content. Removing an ended pane and checkpointing removes its retained history.

Seven real headless checkpoint groups pass: SIGKILL/restart with three stable panes and 5,201 history rows, identical canonical cells/wrapped Unicode, known exit code 23, clipboard-register exclusion and input refusal; idle read-only queries; automatic save/normal stop/explicit durable path with changed runtime socket; durable close; preserved prior archive and surfaced write failure/retry; ordinary XDG state recovery after deleting two runtime trees; and ten ownership/corruption/file-boundary negatives. Two native GTK/Wayland groups pass: the same tab/view IDs reopen after daemon SIGKILL, ended history is visibly labelled and copyable through the existing internal buffer, and a new live terminal works alongside it. Copy keys in this fixture invoke GTK callbacks synthetically; this is not physical-keyboard acceptance.

Existing regressions pass eight workspace groups, nine actual local GUI/API groups, native terminal/GDK checks (PRIMARY remains blocked by the owned compositor) and four genuinely authenticated isolated IronRDP groups, including background clipboard ownership and mixed-view transport retention. The existing build workflow runs the headless Rust units, 100 startup races, state acceptance, the new recovery harness and all 20 native Rust units. A warm bounded fixture measured 30 actual framed checkpoint RPCs/atomic writes/fsync with 5,200 generated Unicode rows: median 17.65 ms, p95 28.83 ms and maximum 28.94 ms. Raw samples and exact commands/hashes remain in the private evidence capsule; this is not matched tmux or idle-host performance qualification.

Final canonical-model review also found that literal tab cells must be accepted; ESC and other control cells remain rejected. The pending release was stopped before publication and its existing allocation retained while the tab regression was added. Retained failed attempts include a missing Rust io qualification in the durable-default-path helper, the first GUI fixture using a schema field name instead of the actual inspector field, and a screenshot requested before the replacement GTK surface had rendered. The final fixture focuses the canvas and waits for a real rendered PNG. The owner desktop was activated for viewing without replacing or terminating it.

```sh
./build.sh --release --test --desktop
python3 ruwac/tests/integration/recovery.py --binary "$CARGO_TARGET_DIR/release/ruwac" --evidence "$RUWAC_EVIDENCE_DIR"
python3 ruwac/tests/integration/recovery_desktop.py --binary "$CARGO_TARGET_DIR/release/ruwac" --gui "$CARGO_TARGET_DIR/release/ruwac-desktop" --evidence "$RUWAC_EVIDENCE_DIR"
./release.sh beta
./release.sh --verify
./build.sh --release --desktop --install-user
```
Graphical tests require an owned compositor; the exact node commands, candidate/released hashes, immutable failures, checkpoint benchmark samples and installation/publication receipts are retained privately in history-r12. Version allocation/publication remains with the existing controller. No shared release infrastructure was modified.


## Current persistent graphical identities and permission-scoped workspace API — 2026-10-11

Each native tab and individual terminal/RDP/Files view now has an opaque ID independent of its label, index, underlying terminal session and RDP worker. IDs survive reparenting, join/separate, view movement and desktop crash/restart. A newly created tab gets a new tab ID; moving a view retains its view ID. Closing a view/tab invalidates that graphical target without terminating persistent terminal processes.

Schema-three private workspace metadata records tab/view IDs and the last focused leaf per tab. Versions one and two migrate with a mode-0600 backup of the original bytes. Duplicate IDs, invalid focus targets and cross-tab focus references fail validation. Restoration retains the exact selected view; removed profiles become ended placeholders with their original view IDs. Focus after reparenting waits for GTK mapping, is bounded and is cancelled when the selected tab changes. The native Views dialog now offers Focus alongside Move and Close.

A separately authenticated GUI socket serves protocol-zero workspace.describe, workspace.list, workspace.activate and workspace.focus. The CLI uses the same private daemon target to find the GUI endpoint, verifies same-UID credentials/correlation and never starts a daemon or GUI for a workspace command. Discovery/listing is read-only; focus mutations require the desktop’s “Allow automation to change workspace focus” permission, which can be revoked at runtime and defaults off each launch. --allow-workspace-control enables it explicitly for that launch. The API returns stable IDs, view kinds, terminal-session references, remembered focus and actual input-focus/window state; it does not expose clipboard contents, credentials or remote input.

The socket uses an owned 0700 parent, mode-0600 endpoint/lock and SO_PEERCRED. It bounds frames to 1 MiB, peers to eight and queue capacity to sixteen. Socket workers do not touch GTK. Input and dispatch have two-second deadlines; queued expired requests cannot change focus. The CLI has a five-second total deadline. A nonblocking startup probe preserves a live listener with a full backlog. Audit records contain actor, operation, valid target ID and verdict, excluding arbitrary request values. Clipboard ownership follows native window activation: focusing a view through IPC while RUWAC is in the background does not grant its RDP session desktop ownership.

Nine compiled native workspace/API groups pass: no autostart/unsafe-path preservation; full-backlog listener preservation without freezing GTK startup; mixed view IDs and native Focus button; permission revocation, malformed/oversized frames, version/correlation, stale/missing targets, eight-peer limits and responsive GTK; expired queued commands; move/close; exact focus after join/separate; crash restoration and stale-socket recovery; and schema-two migration with byte-for-byte backup and stable IDs on the next launch. Terminal PIDs remain unchanged. The API measurement uses 30 actual CLI/process/socket/GTK round trips: median 16.15 ms, p95 32.43 ms, maximum 33.97 ms. This is an isolated-compositor API measurement, not matched tmux or idle-host qualification.

Eight existing workspace groups pass with schema-three metadata. Native terminal/GDK regressions pass 21 checks, with PRIMARY blocked by the compositor. Four authenticated local RDP groups pass, including the new background API test: three live CredSSP sessions cannot replace an independent foreground GDK peer’s clipboard after an authorized background focus command; transfer works again after actual native window activation. Mixed transport reparent/move/close and Files clipboard isolation still pass. All four compositor-delivered Alt-number groups, named CLI regressions and seven safe diagnostic groups pass. The standard build/release runs the headless Rust units, 100 startup races, state acceptance and all 20 native Rust units.

Retained private failures include initial Rust syntax/type errors and the first extended focus test, which exposed GTK mapping timing after separation. The final candidate has bounded deferred focus and separate actual input-focus evidence. Synthetic RDP in the ID fixture is distinguished from the independently authenticated local RDP transport tests. The owner’s existing desktop and terminal sessions are preserved.

```sh
./build.sh --release --test --desktop
python3 ruwac/tests/integration/workspace_api.py --binary "$CARGO_TARGET_DIR/release/ruwac" --gui "$CARGO_TARGET_DIR/release/ruwac-desktop" --evidence "$RUWAC_EVIDENCE_DIR"
python3 ruwac/tests/integration/workspace.py --binary "$CARGO_TARGET_DIR/release/ruwac" --evidence "$RUWAC_EVIDENCE_DIR"
python3 ruwac/tests/integration/native_rust.py --binary "$CARGO_TARGET_DIR/release/ruwac" --gui "$CARGO_TARGET_DIR/release/ruwac-desktop" --evidence "$RUWAC_EVIDENCE_DIR" --primary-unavailable
python3 ruwac/tests/integration/rdp_clipboard.py --binary "$CARGO_TARGET_DIR/release/ruwac" --gui "$CARGO_TARGET_DIR/release/ruwac-desktop" --server "$CARGO_TARGET_DIR/release/examples/rdp-fixture" --evidence "$RUWAC_EVIDENCE_DIR" --mixed-only
./release.sh beta
./release.sh --verify
./build.sh --release --desktop --install-user
```
Graphical acceptance requires an owned compositor and RUWAC_XTEST_DISPLAY. Exact node commands, hashes, failed attempts, latency summaries, publication and installation receipts are retained in the private identities-r11 evidence capsule. The existing release controller owns versioning; no shared release infrastructure was changed.


## Current native shortcuts, appearance and safe diagnostics — 2026-10-11

The native desktop now captures Alt+1 through Alt+9 and Alt+0 at window level, selecting workspace positions 1–10 across terminal, RDP and Files views. Missing positions are a no-op. Ctrl+Alt/AltGr-style combinations, Shift and Super chords are not captured; Caps Lock is allowed. Tab selection retains existing focus/clipboard ownership handling and persistent connections.

The appearance follows the owner’s installed Ptyxis reference: native dark libadwaita chrome, a compact header, and workspace/PNG actions on a separate toolbar. `--theme dark|light|system` selects the desktop appearance (dark is the default). This is incremental styling of the existing compiled GTK client; the terminal engine and application-supplied terminal colors remain unchanged.

`ruwac doctor` provides bounded read-only socket ownership/permissions and daemon-health checks. `--desktop` checks executable/display/session-bus availability, without claiming a working graphical connection from environment variables alone. `--terminal` inspects only stdin’s TTY. Explicit `--terminal --repair` restores canonical input, echo, signals and newline processing only when this process owns the invoking TTY foreground group. Queued input/custom control characters are retained; no terminal escape reset is emitted. Existing paths are never repaired or deleted. Remote read-only diagnostics use the authenticated SSH transport; remote repair is refused.

Seven disposable local diagnostic cases pass, including a one-second stalled-listener bound, preservation of unsafe paths, read-only health/revision, missing graphical dependencies and repair confined to one foreground PTY while a second PTY stays unchanged. Three additional actual authorized SSH diagnostic checks pass: health/revision unchanged, structured no-TTY failure and remote repair rejection. Named CLI regression passes.

Four native shortcut/appearance groups pass using XTest events through isolated nested Weston into the actual GTK capture controller: eight terminal tabs plus an RDP view and Files view, all ten Alt-number positions, missing-tab behavior, modifier guards/Caps Lock, and a PNG exported from the actual dark GTK window. The shortcut fixture’s RDP frame is synthetic, not a remote login. Separate regressions exercise eight workspace persistence groups, native terminal/GDK clipboard checks and three authenticated local RDP mixed-workspace/clipboard groups. Physical GNOME keyboard/AltGr and external Windows interoperability remain separate gates.

Initial harness failures are preserved privately: Python changes the representation of PTY VMIN/VTIME after disabling canonical input, an unsupported named CLI fixture argument, and inspection before GTK window activation. The test now compares attributes read back from the PTY, uses canonical RPC for fixture argv and waits for activation. The toolbar was rebuilt after overlapping source synchronization. The final regression then found that moving PNG staging out of the header cleared the terminal target; the focus handler now preserves the target for both action containers while clearing remote ownership. Native regression is repeated after that fix.

```sh
./build.sh --release --test --desktop
python3 ruwac/tests/integration/doctor.py --binary "$CARGO_TARGET_DIR/release/ruwac" --evidence "$RUWAC_EVIDENCE_DIR"
python3 ruwac/tests/integration/desktop_shortcuts.py --binary "$CARGO_TARGET_DIR/release/ruwac" --gui "$CARGO_TARGET_DIR/release/ruwac-desktop" --evidence "$RUWAC_EVIDENCE_DIR"
python3 ruwac/tests/integration/native_rust.py --binary "$CARGO_TARGET_DIR/release/ruwac" --gui "$CARGO_TARGET_DIR/release/ruwac-desktop" --evidence "$RUWAC_EVIDENCE_DIR" --primary-unavailable
./release.sh beta
./release.sh --verify
./build.sh --release --desktop --install-user
```
Graphical tests require an isolated compositor and RUWAC_XTEST_DISPLAY. Exact node commands, hashes, retained failed attempts, authenticated SSH checks and publication/install receipts remain in the private doctor-r10 evidence capsule. The existing controller owns version allocation and publication.


## Current authenticated host CLI and discovery — 2026-10-11

`ruwac --host SSH_HOST` now sends named commands, raw RPC and JSON stdin requests through authenticated OpenSSH. `--remote-binary` selects the endpoint executable; an explicit `--socket` selects the endpoint socket. Without it, the remote CLI chooses its own private default: the local RUWAC_SOCKET is not forwarded. Existing SSH config and keys are used with BatchMode and strict host-key checking; agent and configured port forwarding are disabled. Request data travels as JSON on stdin, never as a remote shell argument. Executable/socket arguments are individually shell-quoted. Destination/option validation rejects malformed hosts, duplicate globals and remote desktop actions.

`--host HOST start` starts/reuses the remote daemon only when explicitly requested. `--host HOST attach --session ID` opens a real SSH PTY, retaining daemon sessions after detach. Control does not implicitly start a daemon. Remote reply version/correlation and 1 MiB bounds are checked. Control has a 70-second response deadline and a two-second post-EOF exit bound; failure terminates/reaps the SSH process. Interactive attachment is intentionally governed by the user’s session lifetime.

`describe` is an additive read-only daemon method exposing schema, capabilities, version and transport boundaries. The CLI probes the schema first and can aggregate existing read-only methods for an older live daemon, returning native_describe=false without inventing capabilities or restarting its processes. Discovery does not change daemon revision. The daemon/CLI remain independent of GTK, IronRDP and libssh2.

Actual authorized Fedora SSH: three groups pass using the new --host implementation against a generated private daemon/socket. Start/reuse and discovery, exact Unicode/shell-metacharacter file delivery, separate Enter, JSON request correlation, stale/missing target negatives and remote GUI rejection pass. Three genuine remote-PTY attach/type/detach/reattach cycles keep one child PID and negotiate 105 columns/30 content rows. An executable from the tested candidate is used; owner sessions and credentials are untouched.

Eleven synthetic subprocess-contract cases pass separately: normal response, wrong ID/version, structured error, oversized output, malformed JSON, unsuccessful exit, EOF while process remains alive, silent live peer, global-option validation and no forwarding of the local default socket. The silent peer is terminated at 70.01 s; post-EOF termination occurs at 2.04 s. These cases establish failure handling, not remote login.

Named CLI regressions pass locally, including file input and read-only discovery. The standard headless build runs all 18 Rust units, 11 groups/100 startup races and three state groups. Discovery also succeeds against the existing older live daemon with identical health/revision before and after. Native desktop, RDP/clipboard and SFTP acceptance from prior increments is retained; those entire GUI suites were not repeated for this CLI-only change. The standard release still builds/tests the native workspace.

```sh
./build.sh --release --test --desktop
python3 ruwac/tests/integration/named_cli.py --binary "$CARGO_TARGET_DIR/release/ruwac" --evidence "$RUWAC_EVIDENCE_DIR"
python3 ruwac/tests/integration/remote_contract.py --binary "$CARGO_TARGET_DIR/release/ruwac" --evidence "$RUWAC_EVIDENCE_DIR"
python3 ruwac/tests/integration/host_cli.py --binary "$CARGO_TARGET_DIR/release/ruwac" --host "$RUWAC_AUTHORIZED_SSH_HOST" --remote-binary "$RUWAC_REMOTE_BINARY" --remote-receiver "$RUWAC_REMOTE_RECEIVER" --remote-evidence "$RUWAC_REMOTE_EVIDENCE" --evidence "$RUWAC_EVIDENCE_DIR"
./release.sh beta
./release.sh --verify
./build.sh --release --desktop --install-user
```
Exact node commands, hashes, receipts, retained test-cleanup failure and final publication/install verification are in the private remote-r9 capsule. The controller owns release allocation.


## Current individual view management — 2026-10-11

**Views…** opens a native dialog for the selected mixed workspace. Each leaf offers **Move to own tab** and **Close view**. Removing a leaf collapses its divider and keeps the sibling widget/tree alive. Moving preserves the underlying connection; closing a terminal detaches its presentation while daemon programs keep running, closing RDP disconnects only that connection, and closing Files cancels its work and releases its own lease. Closing the last leaf removes the empty tab. Dialog errors remain visible if a view disappeared. The dialog takes local focus before opening; remote clipboard ownership cannot follow it.

Connection registries remove only the selected leaf. Profile deletion protection survives a move and remains until the last associated view closes. Workspace schema 2 records the resulting topology without a schema change; no daemon protocol, engine or dependency replacement was introduced. Existing terminal splits inside a session remain independent of mixed GUI leaves.

Eight native workspace groups pass: live same-PID reopening, migration/backup, profile leases, mixed GUI-crash restoration, separate daemon sockets, unsafe metadata, individual view management and the empty-workspace lifecycle. Actual dialog buttons close a nested terminal leaf, preserve and reopen its daemon PID, move one leaf into a tab and close a sole-view tab. Closing all presentations checkpoints zero tabs while original daemon PIDs survive; New terminal creates a usable view from that empty workspace.

The focused RDP suite passes three groups against actual TLS-verified/CredSSP loopback sessions and real GTK/GDK. A moved RDP leaf retains its transport ID. Closing the selected RDP disconnects only that view; both neighboring transports remain unchanged. A replacement connection authenticates, decodes frames and transfers CLIPRDR text. Existing nested-tree and inactive-offer ownership checks also pass. The full 13-group clipboard/display/editor acceptance remains retained as preceding-batch evidence; it was not rerun in full for this view-management change.

All six real isolated SSH/SFTP groups pass. The native panel moves out of a terminal/Files split, refreshes its real directory listing successfully, and closes while preserving its neighboring terminal PID. SHA-256 integrity, cancellation, slow peer and transport-loss regression groups pass. An in-flight transfer during GTK reparenting is not separately qualified. All 21 native terminal cards pass, with PRIMARY explicitly unavailable on the owned compositor. Failed test assumptions about restored tab kinds, session chooser opening and reordered RDP tabs are retained privately and corrected by locating the actual resource.

Use the existing `build.sh --release --test --desktop`, the workspace/native terminal/RDP/SFTP commands from the preceding section, and `release.sh beta`/`release.sh --verify`. The focused RDP rerun adds `--mixed-only`. Tests require an owned compositor and disposable fixtures. Exact commands, candidate hashes, retained failures, build/release logs and final publication/install receipts are in the private views-r8 capsule; the release controller assigns versions.


## Current mixed workspace increment — 2026-10-11

Native **Split tabs…** joins existing terminal, RDP and Files views into nested horizontal/vertical GTK Paned trees. **Separate views** returns the leaves to tabs without replacing terminal PTYs or RDP transports. Profile deletion leases survive reparenting. RDP controls now live in a popup, leaving room for the framebuffer in mixed splits.

Clipboard ownership follows the focused terminal canvas or RDP canvas, rather than the enclosing tab. Files and local controls clear remote ownership and release held RDP input. Stage clipboard PNG remembers the focused terminal across the toolbar click, refocuses it before staging, and keeps a visible success/error message. Inactive RDP offers remain isolated.

Private workspace schema 2 records all tab kinds, nested split orientations/ratios, selected tab, nonsecret RDP configuration and Files directories. A 750 ms checkpoint and final close save support GUI-crash restoration while the daemon remains alive. Schema-1 migration preserves a mode-0600 original backup. Other daemon sockets use separate hashed metadata files. Restoration reuses live terminal IDs; missing sessions become ended placeholders. RDP restores disconnected; Files restores without silently authenticating or refreshing. Current library identities govern profile binding and deletion leases. No password, terminal output or clipboard payload is serialized.

The owned nested Wayland compositor exercises real GTK/GDK. Six workspace cases cover live same-PID reopening, schema-1 migration with private backup, profile leases during reparenting, nested terminal/RDP/Files restoration after GUI SIGKILL, separate socket metadata and unsafe symlink preservation. The restored RDP metadata test uses a disconnected placeholder; it does not establish remote login.

All 13 full RDP groups pass against disposable authenticated IronRDP servers, including the new live mixed-tree test. Three authenticated transports keep their connection IDs during join/separate; actual CLIPRDR text and inactive offers are verified while mixed. After compacting controls, six editor/keyring/display/Files groups and three authenticated mixed/ownership groups are rerun. The isolated Secret Service provider performs real credential lookup for normal sidebar connections. These are real local TLS/CredSSP, framebuffer and clipboard integrations, not external-host acceptance.

Six SFTP groups pass with real isolated SSH transport, SHA-256 integrity, cancellation, slow peer/loss and native panel listing. Native terminal acceptance retains 21 passing cards, actual Vim/less/htop, independent GDK clipboard consumers and actual toolbar PNG staging followed by GUI-crash cleanup; PRIMARY is separately unavailable on this compositor. Reparenting an active file operation is preserved by implementation, but not yet separately qualified during an in-flight transfer.

Matched API benchmarks use 30 alternating samples for each action at 1/5/20/40 panes. All 12 median ratios are within 2× tmux; the VM was 25.7% busy, so idle-qualified acceptance remains open. Key-to-emitted-render median/p95 is 36.16/38.12 ms, with a 5 ms observer; this is not physical display latency. At 40 panes daemon/TUI RSS is 13.82/5.22 MiB, excluding child interpreters. The independent daemon/CLI source is preserved.

```sh
./build.sh --release --test --desktop
cargo build --locked --release --manifest-path ruwac/desktop/Cargo.toml --examples
python3 ruwac/tests/integration/workspace.py --binary "$CARGO_TARGET_DIR/release/ruwac" --evidence "$RUWAC_EVIDENCE_DIR"
python3 ruwac/tests/integration/native_rust.py --binary "$CARGO_TARGET_DIR/release/ruwac" --gui "$CARGO_TARGET_DIR/release/ruwac-desktop" --evidence "$RUWAC_EVIDENCE_DIR" --primary-unavailable
python3 ruwac/tests/integration/rdp_clipboard.py --binary "$CARGO_TARGET_DIR/release/ruwac" --gui "$CARGO_TARGET_DIR/release/ruwac-desktop" --server "$CARGO_TARGET_DIR/release/examples/rdp-fixture" --evidence "$RUWAC_EVIDENCE_DIR"
python3 ruwac/tests/integration/sftp.py --driver "$CARGO_TARGET_DIR/release/examples/files-fixture" --binary "$CARGO_TARGET_DIR/release/ruwac" --gui "$CARGO_TARGET_DIR/release/ruwac-desktop" --evidence "$RUWAC_EVIDENCE_DIR"
./release.sh beta
./release.sh --verify
./build.sh --release --desktop --install-user
```
GUI tests require an owned compositor, private test D-Bus and disposable keyring; fixture credentials never enter public receipts. Exact build commands, hashes, benchmarks and retained failed trials are in the private mixed-r7 evidence capsule. Version allocation and publication remain the existing release controller’s responsibility.

## Earlier milestone evidence

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
python3 ruwac/tests/integration/workspace.py --binary "$CARGO_TARGET_DIR/release/ruwac" --evidence "$RUWAC_EVIDENCE_DIR"
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
