# RUWAC acceptance status

Public results summarize qualification without publishing host identities, accounts,
network topology, credential references or operational troubleshooting. Detailed
receipts and original reports are archived privately outside Git.

| Area | Result | Scope |
| --- | --- | --- |
| Split-pane exit collapse | PASS | Twelve real PTY/native GTK groups against current and isolated older daemons: two concurrent TUIs, GTK-only cases, full viewport, repaired focus, exact input bytes, nested survivors and attachment after exit. Completed split history follows ordinary pane-close semantics; final-session and restored history remain inspectable. |
| Terminal shell startup and exit | PASS | Real PTYs, native GTK and an isolated older daemon: explicit login-shell argv in new tabs/windows/splits, real Bash profile/rc, live-window fallback, last-view closure and retained ended history |
| GTK Ctrl+A routing | PASS | Real tmux configured with Ctrl+A receives a single prefix; local RUWAC GTK prefix is Ctrl+Alt+A; CLI prefix remains Ctrl+A |
| CLI status controls | PASS | Actual bottom-row SGR clicks select windows; lowercase horizontal and vertical split shortcuts create persistent panes; nine CLI regression groups |
| Reconnect framebuffer dimensions | PASS | Initial client-size-aware authenticated loopback peer, three reconnects at 1400×868, bounded trace of decoded sizes excludes transient smaller frames after settling; external Windows window-placement remains unverified |
| Saved-profile reconnect controls | PASS | Four mapped editor/Save & Connect cycles, centered disconnected overlay without viewport change, targeted tab reconnect and preserved neighbor transports; synthetic unmapped-dialog GTK limitation remains documented |
| About and donation link | PASS | Native dialog construction; independently decoded static QR points to dedicated public support page; no payment submission, invoicing or payment credentials in RUWAC |

| Scrollable, collapsible machine groups | PASS | 123 generated profiles without pagination, native disclosure, search reveal/restore, preference preservation and restart persistence |
| RDP drag release and viewport stability | PASS | Actual authenticated loopback left/right release receipts, GTK gesture cancellation, tab-switch release, zoom remapping and preserved pan; synthetic input, external physical reproduction remains separate |
| Bulk Remmina metadata import | PASS | Generated native/legacy/Flatpak profiles; confirmation, cancellation, changed-file refusal, duplicate skipping and credential exclusion |
| Dynamic RDP and local host shortcuts | PASS | Owned Mutter 50.5, twenty Display Control reactivations and allocation-driven resize storm; real GDK Ctrl+Alt+Home and Ctrl+Alt+Enter events release focus and enter/exit fullscreen without reconnecting |
| Header Controls and optional dock | PASS | 32px native header; popover preserves terminal/RDP allocation, explicit dock takes layout space; synthetic GTK callbacks |
| Screen CLI aliases and readable session list | PASS | Eight real PTY groups including full-width bottom status through normal and beyond-limit physical resizes, detached/shared sessions, automation title, control-key navigation and kill cancellation; full GNU Screen language remains unsupported |
| Desktop appearance and font metrics | PASS | Six actual rendered background pixel checks, saved schema-2 preferences and PTY dimensions changed by font size; unavailable fonts fall back to Monospace |
| Remote agent launch configuration | PASS | Native SSH profile saves commands/backend; generated command/quoting tests; no real agent account or external remote installation claimed |
| Persistent remote agent session reuse | PASS | Seven actual loopback SSH/PTY groups: concurrent creation, shared input, detach, same-process reattach, independent agents, ended sessions, duplicate/unsafe-lock refusals; generated agent only |
| Remote CLI installation and external agent launch | BLOCKED | Explicit installer/launcher implemented; external host prerequisites and agent login remain separate acceptance |
| Weston popup-after-resize compatibility | BLOCKED | Weston 15.0.1 exited in popup positioning on two owned displays; Mutter qualification is recorded separately; no physical desktop pass inferred |
| Strict warnings-as-errors desktop lint | BLOCKED | Local Clippy component unavailable; existing GTK deprecation/dead-code warnings remain; ordinary hosted lint is a separate quality gate |
| Current automatic GDM sign-in | BLOCKED | Owner reports failure in older running GUI; current recognition regressions pass, but no new real-host login claimed in this batch |
| Named CLI creation/reattachment, window titles and concurrent clients | PASS | Actual generated local PTYs; exact-name and recursion safeguards; two simultaneous clients |
| Nested prefix routing | PASS | Two through five real RUWAC layers and native GTK with actual tmux; synthetic GTK callbacks |
| Remmina default discovery | PASS | Generated native/legacy/Flatpak profiles; read-only source and explicit metadata import; credentials excluded |
| Verified opt-in system RPM updates | PASS | Official signed workflow provenance, actual Fedora DNF, generated fault tests, native GTK controls and isolated system timer; physical authorization remains separate |
| Persistent terminal daemon, CLI, Screen navigation and history | PASS | Rust units and real generated PTY fixtures |
| Independent attached-client selection, zoom and input | PASS | Disposable daemon, two real TUI clients, generated raw receivers; shared PTY sizing, closed-target repair and scoped/global deduplication |
| Native GTK profiles, SQLite migration and credential separation | PASS | Generated profiles, disposable Secret Service, isolated compositor |
| RDP authentication, framebuffer, text/PNG clipboard and focus ownership | PASS | Authenticated IronRDP loopback fixtures with generated credentials |
| Fedora RDP sign-in and bidirectional text/PNG clipboard | PASS | Authorized Fedora RDP host; GNOME Remote Desktop 50.2 / GDM 50.3; simulated local input |
| Windows RDP authentication and framebuffer | PASS | Windows test host; single authenticated connection |
| RDP latency, CPU and memory comparison | PASS | Authenticated same-endpoint RUWAC/FreeRDP X11 fixture, ten alternating trials and fifty actual observed frames per client; external/physical and 3K scopes remain separate |
| Windows dynamic display and external clipboard matrix | BLOCKED | Qualification incomplete |
| Single-operation large native clipboard paste | PASS | Actual 1 MiB GDK transfer and daemon/PTy Unicode fixture; one bracket pair, staging/ownership/limits/expiry; synthetic GTK callback |
| Desktop clipboard deadlines and stalled-provider cancellation | PASS | Real GDK/GIO on owned Wayland; generated incomplete provider, kernel-stream recovery and partial-file cleanup |
| SSH/SFTP integrity, progress, cancellation and persistence | PASS | Isolated authorized fixtures; generated files and keys |
| Explicit SSH jump routing and separate gateway/target trust | PASS | Two disposable OpenSSH servers; eight actual transport groups and four native editor groups |
| Remote agent image transport/consumption | PASS | External CLI route and native loopback route; complete external native launcher remains unqualified |
| PRIMARY and native middle-click dispatch | PASS | Real GDK/Mutter protocol, generated text, synthetic click dispatch, SGR routing and Shift bypass; physical input remains separate |
| Physical GTK/Wayland mouse, keyboard and touchpad acceptance | BLOCKED | Owner confirms Alt+1–0 on 0.2.23; owner also reports local desktop selection/Shift+drag copying (version unspecified); middle-click, touchpad and complete newer-version physical checks remain unqualified |
| Upstream Fedora testing RPM payloads and private transactions | PASS | Actual separate core/desktop packages, executable versions, launcher/notices and prior-release upgrade/removal; private RPM database uses host libraries and --nodeps |
| Clean Fedora desktop DNF install/upgrade/rollback/remove | PASS | Fresh pinned Fedora 44 RAM filesystem, actual published RPMs, real dependency resolution without --nodeps; daemon, PTY child and generated private configuration survive |
| RPM signing/repository and complete binary security qualification | BLOCKED | Workflow provenance is distinct from a signed Fedora repository; desktop RSA advisory remains open |
| Desktop rustls security patch | UPDATED | Lockfile pins upstream-patched 0.23.46 for RUSTSEC-2026-0346 through RUSTSEC-2026-0349; authenticated TLS, clipboard and certificate regressions are mandatory release gates. The independent RSA finding is not suppressed. |
| Desktop RSA dependency advisory | FAIL | RUSTSEC-2023-0071 remains unsuppressed |

These results describe the published implementation baseline. In-progress codec
experiments do not establish Windows resize acceptance or full stable qualification.

Reproduce portable tests after building into a separate build directory:

```sh
cargo test --locked --workspace --manifest-path ruwac/Cargo.toml
cargo test --locked --manifest-path ruwac/desktop/Cargo.toml
python3 ruwac/tests/integration/client_focus.py --binary "$CARGO_TARGET_DIR/release/ruwac" --evidence "$EVIDENCE_DIR"
python3 scripts/test-privacy-check.py
python3 scripts/public-files.py --tracked
python3 ruwac/tests/integration/test_gdm_recognition.py
python3 ruwac/tests/integration/test_updates.py
```

Native authenticated clipboard fixtures are driven by
`ruwac/tests/integration/rdp_clipboard.py --help`. They generate their own loopback
server, disposable credentials and isolated clipboard state. Never point synthetic
credential or destructive fixture tests at production accounts.

Build and publication use the existing `build.sh` and `release.sh` adapters; the
installed controller assigns versions. GitHub source archives contain source,
not a claim of qualified Fedora binaries. Historical references are audited
separately and remain unchanged pending owner approval.

Reproduce PRIMARY and click dispatch with test-only Mutter, D-Bus and Python GObject dependencies:

```sh
python3 ruwac/tests/integration/primary_wayland.py \
  --binary "$CARGO_TARGET_DIR/release/ruwac" \
  --gui "$CARGO_TARGET_DIR/release/ruwac-desktop" \
  --evidence "$EVIDENCE_DIR" --spawn-owned-display
```

The fixture creates and stops its own compositor and child processes. Its
`--owned-display` mode refuses the normal user desktop runtime and requires an
explicitly named test display. Synthetic callbacks qualify protocol and dispatch
behavior, not physical mouse or touchpad operation.

One combined RDP stress run timed out during reconnect and resize. Isolated and
repeated combined tests passed. The cause of that intermittent failure has not
been established; it remains recorded in private qualification evidence.
