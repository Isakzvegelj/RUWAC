# RUWAC component provenance

## Mixed GNU Screen/tmux terminal qualification — 2026-10-11

Compiled package 0.1.17 preserves the existing engine and adds a reproducible real-PTY mixed compatibility matrix to normal build tests when GNU Screen/tmux test tools are present. Missing tools explicitly block this qualification without adding daemon runtime dependencies. Sixteen runs cover eight two-to-five-layer chains with both common Control-A and native tmux Control-B prefix configurations, including each tool as an outer client. They verify exact Unicode/combining/emoji bytes, both literal prefixes, deep window creation/selection, single-pair multiline bracketed paste, exact width/height resize propagation, outer detach/reattach and unchanged leaf PID/start time. The owned PTY modes are restored. These are local private multiplexer servers, not simulated nesting.

A separate seventeenth case reproduces GNU Screen 5.0.1 interpreting its own Control-A detach command inside a bracketed pasted payload. The outer RUWAC paste remains literal, but Screen ends its attach client and truncates delivery at its command prefix. This is an explicit observed upstream limitation, not a passing arbitrary-control-paste gate. tmux/RUWAC chains do contain both Control-A/Control-B command prefixes in paste. Addressing a known leaf RUWAC daemon through its explicit pane API bypasses intermediate key parsers; RUWAC does not rewrite unknown Screen state or promise arbitrary-control paste through Screen. Physical keyboards and mixed paste through other upstream versions remain unqualified.

The fixture owns separate HOME/configuration, Screen directory, tmux sockets, RUWAC daemons and raw leaf receivers. No owner sessions/configuration are touched. Initial harness failures included a non-atomic resize receipt and reentrant signal writes; the corrected per-chain atomic receipt serializes resize handlers. Failed receipts remain private. Actual versions: Screen version 5.0.1 (build on 2026-01-05 00:01:00); tmux 3.7c. Existing headless copy/history/recovery and native library/GTK/RDP/SSH regression suites repeat on the stamped build.

CLI SHA-256: `0042c64372aa114229dc7120d6dc8c3502728e55e0e4802ea0579e226d37d0f4`. GTK SHA-256: `9bb3e04b1826f18c809e29696372e27fbf2a545edcf6d38c7a8687087857c094`. Build: `build.sh --release --test --desktop`; publication: existing `release.sh beta`, controller-owned versioning. The full product remains incomplete: external RDP, physical desktop, VT/extended-input, performance/soak, agent/file/automation and qualified Fedora distribution remain in the completion audit.


## Usable native machine sidebar — 2026-10-11

Compiled package 0.1.17 keeps saved machines prominent beside terminal/RDP workspaces. Search/filter, creation and paging remain visible; native “Machine tools” and “Import from Remmina” panels start collapsed. Maintenance actions, explicit read-only import preview, identity review and Undo retain their original handlers. The expanded import area scrolls within a 280-pixel natural-height limit. Nonempty status messages wrap within a bounded text width and support selection; empty status does not consume layout space. Existing profile files, credentials and live connections are preserved.

Eight actual GTK fixture groups pass. In the 1,000-profile fixture the collapsed layout assigns 404 of 636 allocated sidebar pixels to the machine list. Native expanders open/close, all endpoints remain reachable across 20 pages, and reviewed merge/Cancel/Undo, stale/active guards and restart pass. This measures actual GTK allocation on the owned compositor with synthetic widget actions, not physical keyboard/touchpad or accessibility qualification. Full fifteen-group authenticated local RDP regression repeats real text/PNG transfer, three-tab ownership, lifecycle permissions, clipboard isolation, display changes, profile editing and private Secret Service lookup. Existing headless/native/SSH regressions also repeat on the stamped artifacts.

CLI SHA-256: `0042c64372aa114229dc7120d6dc8c3502728e55e0e4802ea0579e226d37d0f4`. GTK SHA-256: `9bb3e04b1826f18c809e29696372e27fbf2a545edcf6d38c7a8687087857c094`. Build: `build.sh --release --test --desktop`; source publication: existing `release.sh beta`, with controller-owned versioning. The actual GTK screenshot uses the generated authenticated test framebuffer and is not an external Windows/Fedora login. Full product and stable qualification remain incomplete; the product completion audit retains external/physical, agent/file/automation and qualified Fedora distribution gates.


## Permission-scoped native RDP lifecycle and SSH GUI transport — 2026-10-11

Compiled package 0.1.17 exposes read-only `rdp list/query`, saved-profile `rdp open`, and explicit-view `rdp reconnect/disconnect` through the existing authenticated native GUI actor. RDP control requires separate owner consent, disabled by default; permission revocation applies at dispatch. Activation additionally requires the existing focus permission. Background open/reconnect preserves the selected tab, neighboring transports and desktop clipboard ownership. Credentials resolve inside the existing Secret Service worker, never in CLI arguments or API results. The response acknowledges connection startup; callers query phase and decoded updates to establish actual readiness.

Typed stable endpoint/view/tab targets, exact parameter validation, a 64-RDP-view cap, stale/detached target rejection and explicit refusal of idempotency keys bound the operation. Metadata contains phase, framebuffer dimensions, transport identity, decoded-update count and broker ownership; it excludes credentials, certificate paths, pixels and private error text. Audit records contain fixed operations, valid typed identifiers and verdicts without arbitrary payloads. Disconnect leaves the view available for reconnection; closing removes it and makes its identifier stale.

The headless CLI uses the same typed request over verified SSH with `--host`, bounded framed replies and existing trusted same-UID GUI sockets. Neither a missing remote GUI nor a failed tunnel launches a GUI/daemon or falls back to the headless endpoint. SSH authentication and endpoint trust retain existing checks.

Fifteen actual authenticated disposable local RDP groups pass, including generated credentials resolved by the private Secret Service, CredSSP/framebuffer updates, background disconnect/reconnect, independent focus consent, runtime revocation, preserved neighboring connections/GDK clipboard, rejected credential/unknown parameters, payload-free auditing and stale views. Three actual SSH groups pass through EVF and an owned Unix socket forward to a locally rendered CVF GTK fixture. This proves remote CLI transport and permissions; it does not prove graphical rendering on EVF or external RDP login. Eight library, existing native workspace/focus/recovery, headless PTY/history and GTK regressions repeat on the final stamped artifacts. PRIMARY remains blocked on the owned compositor.

CLI SHA-256: `0042c64372aa114229dc7120d6dc8c3502728e55e0e4802ea0579e226d37d0f4`. GTK SHA-256: `9bb3e04b1826f18c809e29696372e27fbf2a545edcf6d38c7a8687087857c094`. Build: `build.sh --release --test --desktop`; publication: existing `release.sh beta`, with versioning owned by the controller. Stable qualification remains open for external Windows/Fedora login/clipboard, physical GNOME input, full automation/agent/file scope and qualified Fedora binary packaging. Host inventory, workspace open/restore, GUI event subscriptions and RDP input automation are not implemented by this increment.


## Machine library paging and reviewed merge — 2026-10-11

Compiled package 0.1.17 adds accessible previous/next controls and a 50-profile page limit. A 1,000-profile native fixture reaches every endpoint across 20 pages; search/filter changes reset the page and refresh clamps it. Empty group labels are bounded separately; group editing remains available. Endpoint IDs, native connect/edit/favorite handlers and complete search remain intact.

Remmina metadata can now merge only into an explicitly selected existing endpoint. The owner reviews a native field-by-field dialog before Apply. Imported name, full group, protocol, server, username and tags change; endpoint/machine identity, credential reference, trust/key paths, preferences, favorites, recency and inheritance remain local. Original Remmina files and excluded passwords/key paths are untouched. The UI warns to verify changed destinations before connecting. Active endpoints are protected, and Apply rechecks both identity/content and active usage after review. Cancel writes nothing, failed persistence restores in-memory metadata, and existing Undo restores the previous private library. Changing the selected source path invalidates the preview.

Eight actual GTK fixture groups pass: compact default sidebar/native panel toggling, 1,000-profile paging/search, explicit target and native review/cancel, persistence and one-endpoint-only changes at the library cap, Undo, stale review, active endpoint rejection and restart. Actions are synthetic widget/dialog callbacks on an owned compositor; no physical input or external login is inferred. Twenty-one GTK unit tests pass, including policy/identity merge preservation. Fourteen authenticated local RDP regression groups passed on the historical library increment before its stamp, including the existing 70-profile import check updated to assert the visible page cap, real text/PNG clipboard ownership and resizing. The full fifteen-group RDP clipboard/lifecycle/editor/display suite is repeated on the final stamped build. Headless canonical copy, nesting, immutable history, daemon recovery and the existing GTK regressions are repeated by the normal stamped workflow.

CLI SHA-256: `0042c64372aa114229dc7120d6dc8c3502728e55e0e4802ea0579e226d37d0f4`. GTK SHA-256: `9bb3e04b1826f18c809e29696372e27fbf2a545edcf6d38c7a8687087857c094`. Compile/test: `build.sh --release --test --desktop`. Publication: existing `release.sh beta`; controller owns versioning. Builds and raw receipts remain in the build workspace. Source publication includes no machine-specific paths or agent instructions.

This advances F-01/F-02; it does not complete inherited network route execution, external RDP/physical GNOME acceptance, agent/file/automation milestones or Fedora binary/RPM qualification. Full product completion remains unproven.


## Headless Screen and immutable copy increment — 2026-10-11

Compiled package 0.1.17. The existing daemon, authenticated protocol, GTK frontend and RDP engine remain the foundation. Headless copy now uses canonical immutable history pins, Unicode grapheme/cell selection, rectangular and soft-wrap copying, search, word/page navigation and non-destructive cancellation. The terminal adds a window chooser, command prompt, help, zoom and configurable Control prefixes/key mappings. All modal mouse/paste input is contained; original terminal modes are restored. Configuration errors are rejected before raw mode.

The normal build workflow includes nine real-PTY acceptance groups: Unicode internal copy/paste, rectangle and physical wrap, oversize/cancel preservation, chooser/prompt/help/zoom, prefix configuration and actual nested RUWAC layers two through five. This is RUWAC nesting evidence; it does not claim GNU Screen/tmux combinations or physical keyboard qualification. Three authenticated API groups prove frozen output across live output and actual resize, owner/pane binding, idempotency-cache bypass refusal, eight-pin global cap, one pin per connection, unchanged terminal revision, disconnect reclamation and actual 30-second expiry. A terminal unit verifies immutable Unicode rows across feed/resize and the 50,000-row tail bound. The first rectangle fixture mistakenly searched common text; the corrected unique search passes, and the failed receipt remains private.

Limits: 50,000 physical rows or 8 MiB per pin, 128 KiB per row/page text, eight global pins, one per connection, 30-second idle expiry; removed tail rows are explicitly reported. Internal selection remains capped at 256 KiB and preserves the prior buffer on failure. Snapshots never enter recovery checkpoints. GTK history reads use pins when supported and preserve older-daemon fallback.

The stamped native build repeats actual older-daemon compatibility, two native recovery groups, eight workspace groups, nine GUI API groups, 21 GTK/GDK terminal cards and fifteen authenticated isolated RDP clipboard/lifecycle/editor/display groups. PRIMARY is unavailable on the owned compositor. Earlier full 14-group loopback RDP text/image/ownership evidence remains historical; this increment makes no external Windows/Fedora login claim. Local compilation uses `build.sh --release --test --desktop`; publication uses the existing `release.sh beta`, whose controller owns the version. GitHub currently publishes source, with no binary compilation or RPM asset qualification.

CLI SHA-256: `0042c64372aa114229dc7120d6dc8c3502728e55e0e4802ea0579e226d37d0f4`. GTK SHA-256: `9bb3e04b1826f18c809e29696372e27fbf2a545edcf6d38c7a8687087857c094`. Full stable qualification remains open: external RDP, physical GNOME/PRIMARY/HiDPI/accessibility, mixed-multiplexer nesting, agent/file/automation scope and qualified Fedora packaging. Owner sessions and configuration are preserved.


## Prior RDP presentation and Screen correction — 2026-10-11

Remmina supplies a visual reference only: searchable/grouped profiles and accessible native connection/display icons. Standard Adwaita theme icons are used, without copying upstream code or assets. Rust/IronRDP remain the existing foundation. Reference: https://remmina.org/screenshots/.



## Validation for prior RDP increment — 2026-10-11

Compiled package 0.1.12: the normal build/release workflow runs standard Rust/headless regressions, including 11 groups and 100 startup races, seven recovery groups and two real-PTY previous/literal/checkpoint-migration groups. Authorized SSH repeated both previous-window groups against a private endpoint candidate; no live session was replaced. The final native build also passes an actual older-daemon compatibility case: unsupported previous-window capability is visible and subsequent input reaches the same child, without freezing navigation.

The stamped candidate passes two native recovery groups, eight workspace groups, nine GUI API groups and 21 native terminal/GDK cards; PRIMARY is explicitly blocked on the owned compositor. Fourteen authenticated local RDP groups pass in the full presentation regression, including 60 text transfers, 18 PNG/DIBV5 cases, 200 tab switches/450 inactive offers, 20 negotiated resizes plus the GTK resize storm, editor/keyring and 1,000-profile checks. Six editor/display/RDP groups are repeated against the final stamped build. Actual visible toolbar callbacks reconnect using a disposable Secret Service credential and verified CA, decode a new framebuffer and preserve both neighboring transports; zoom/fit and unknown-action behavior pass. The actual GTK render is retained privately, with generated server pixels distinguished from external Windows login.

CLI SHA-256: `129d2d784653fd67238ca93cef11eb858c690f530a2d43f1f89687aa98e4ac07`. GTK SHA-256: `9515d9645262c6badcf00d484ee4386efc629d0b699e82d85a1755c277fcb9b2`. Builds and raw fixtures stay in the build workspace. Original source is Apache-2.0. Standard theme icons are resolved from the native desktop; no upstream artwork or code was copied. Earlier failed compilation receipts are preserved, including a fixture JSON GString conversion corrected before publication.

Stable is still unqualified: authorized Windows/Fedora login and clipboard/display/input, physical GNOME/PRIMARY/HiDPI, full headless selection/chooser/nesting, agent/file/automation milestones and Fedora package distribution remain open. `build.sh --release --package rpm` returns 78 with “RPM candidate packaging is blocked: no qualified app-linux package adapter installed.” Exact private feedback is recorded for 0rig; the node relay remains unavailable. Shared release infrastructure and versions were not edited. GitHub publishes source/tag pages; local build.sh compiles and the private installer deploys binaries.


## Current private daemon checkpoint and ended-history recovery — 2026-10-11

The established Rust engine, canonical Alacritty terminal model and native GTK frontend remain the foundation. Recovery adds private checkpoints to those components; it does not replace the engine or repeat market analysis.


## Current persistent graphical identities and permission-scoped workspace API — 2026-10-11

The retained Rust daemon, existing GTK/libadwaita tree and IronRDP implement persistent graphical targeting and local control. No engine replacement or repeated market research was performed.


## Current native shortcuts, appearance and safe diagnostics — 2026-10-11

The existing Rust engine, GTK/libadwaita frontend and IronRDP remain the foundation. The owner’s installed Ptyxis supplies an appearance reference; no terminal replacement or market reevaluation was performed.


## Current authenticated host CLI and discovery — 2026-10-11

Existing Rust engine and OpenSSH provide host control without new dependencies or market reevaluation. GTK/IronRDP/libssh2 remain unchanged.


## Current individual view management — 2026-10-11

Retain the existing Rust engine, native GTK, IronRDP and libssh2. This increment uses existing GTK widgets and resource registries; no market reevaluation or dependency addition.


## Current mixed workspace increment — 2026-10-11

No new market investigation or component substitution. Existing GTK Paned and IronRDP transports provide mixed views; existing private metadata primitives provide restoration. No dependency additions.

## Earlier milestone evidence

No new market evaluation or engine replacement. Existing GTK/libadwaita 0.11.5/0.9.2, IronRDP 0.17.0/session/connector and CLIPRDR remain. Display Control 0.8 uses a bounded framing adapter with an encoded-PDU regression. The dev-only IronRDP server 0.13 generates lossless bitmap fixtures and real reactivation.

SFTP reuses [ssh2 0.9.6](https://docs.rs/ssh2/0.9.6/ssh2/) (MIT OR Apache-2.0) and libssh2 (BSD-3-Clause). Secret Service uses the installed GIO/libsecret-compatible secret-tool instead of a custom vault. Locked package declarations and native component boundaries are recorded in architecture/dependency-licenses.json; this is not binary-distribution license clearance.

External Windows/Fedora-GNOME RDP and physical GNOME PRIMARY/AltGr/IME/touchpad acceptance remain BLOCKED: RDP credentials, matching certificate identity and physical-seat qualification remain unavailable; designated hosts now include an SSH-accessible Fedora target and a Windows target. Loopback authentication, GDK and server-driven reactivation are real integration results, not external-host acceptance. The raw bitmap fixture does not qualify the GNOME/RemoteFX degradation described in [IronRDP #447](https://github.com/Devolutions/IronRDP/issues/447).

Known product limits: gateway/network fields are metadata, not a routing engine; Remmina conflicts support skip or new identity, without merge-by-ID; group default editing covers display/clipboard while other endpoint overrides remain explicit. Profiles used by connection tabs are deletion-protected until all their active tabs close; reference-counted leases release protection without an application restart. SSH terminals use OpenSSH's interactive verification/authentication; Files uses the configured key/agent or Secret Service password, and refuses unknown/changed keys. Encrypted private-key passphrase UI, rsync, recursive transfers and resume are not implemented. Portable SFTP v3 replacement preserves the old file through a backup/restore sequence but is not atomic. A lost connection can leave a named remote partial or backup; connected cancellation cleans its partial. Concurrent changes to a remote namespace are not an atomic SFTP snapshot. Native file chooser upload/download gestures and locked-keyring dialogs still need interactive desktop acceptance; transport integrity uses the production worker through an isolated driver, while panel listing/refresh uses the actual GTK window. File/HTML CLIPRDR, consumer acknowledgement for agent attachments, general termios recovery beyond Bash and daemon/host-crash persistence remain open. RPM/binary publication and product distribution licensing remain unqualified.


## Usable desktop and authorized SSH acceptance — 2026-10-11

No component substitution or renewed market investigation. The retained Rust engine, GTK, IronRDP and libssh2 continue to provide the tested implementation. Local installer and workspace persistence use standard library/existing dependencies.
