# RUWAC product completion audit

## Automatic GNOME sign-in — compiled package 0.2.6

Machines → Edit → GNOME Remote Login → GNOME User Login now offers **Sign in automatically to the saved Linux account**. This optional endpoint setting preserves existing profiles and defaults off. RDP Service Login still authenticates the transport; the independently stored Linux username/password belongs only to GDM. Windows and Desktop Sharing retain their existing single-stage flow.

The native frontend recognizes supported English Fedora GDM forms from its canonical authenticated remote framebuffer. It selects Not listed, submits the saved Linux username, recognizes the focused Password entry, retrieves the separate Linux secret from Secret Service and sends paired native RDP Unicode/Return events. No clipboard, credential CLI arguments or automation response carries the password. Each action is bound to the current saved profile, endpoint/trust, connection, display generation, selected tab, window/canvas focus and input epoch. Unknown/ambiguous forms do not authorize credentials. User input, focus/tab/profile changes, reconnects and unsupported recognition stop delivery. One password attempt is allowed; missing credentials are refused, and no second attempt is inferred from an unchanged GDM prompt. Already transmitted input cannot be recalled.

Real authorized Fedora 44 acceptance passes one-Connect automatic Linux account selection and password submission, a subsequent authenticated same-endpoint handover and rendering of the existing Linux desktop. The local GTK/Wayland seat is isolated and synthetic; the remote service, GDM and Linux desktop are real. Clipboard is disabled, the SSH-attested original TLS leaf remains mandatory, and remote configuration/owner profiles/existing Linux sessions are preserved. Dynamic resolution also follows the authenticated user-desktop handover and converges to the real requested viewport. Stamped probe duration: 11.724 seconds, including process startup and connection establishment. Real GDM tab switching cancels before password submission; a missing Linux credential stops without a password or desktop handover. Native checkbox off/on persistence preserves both credential references and endpoint identities.

All 25 frozen quality/native/file/RDP checks and 66 desktop units pass. Nine local recognition guards exercise missing OCR, unknown/ambiguous fields, insufficient confidence, malformed data, unsupported dimensions and deadline/child cleanup. Existing dual-credential tests retain wrong service credentials, isolated keyring-unavailable cases, stale profile/focus/reconnect refusal and Windows behavior. Wrong real GDM passwords were not deliberately submitted to the owner account; that acceptance gate remains open. CLI SHA-256: `b86eee086bb1979e8d7fd6b46d189872f262d0c2d1da327639146989e57b5f28`. GTK SHA-256: `8af9bdc7b3bcef79bebf0a3bcb61ba0ca764493444f4bb2bcf5c70779423e081`. Exact commands: existing `build.sh --release --test --desktop` and controller-managed `release.sh stable`, with the owned native acceptance runner.

This is targeted client input automation, not a new GDM/RDP authentication API or system-wide autologin. Automatic background-tab login, other GDM layouts/locales, physical-seat input, external clipboard, unfixed RSA dependency security and qualified Fedora RPM/binary distribution remain separate gates. Automatic sign-in requires local Python/Pillow, Tesseract and English language data; unsupported/missing recognition stops safely. No remote Fedora settings were changed.

## Previous graphics/handover baseline, before automatic sign-in

GNOME modes now use the pinned upstream non-AVC EGFX client/compositor and ClearCodec, Planar and RemoteFX Progressive decoding. Standard/Windows connections preserve the existing rendering path. GFX output is applied to the canonical pointer-aware framebuffer. Graphics reset invalidates stale assisted-input generations. The headless daemon retains no graphical dependencies.

The initial service login uses the existing saved username/password and CredSSP. Enhanced GNOME redirection is parsed with bounded fields; only encrypted, same-endpoint handover is accepted. The next connection negotiates RDSTLS using opaque server-issued credentials, with both original endpoint trust and the previously authenticated TLS leaf required. Supplied destinations/certificates are never adopted. Routing tokens, encrypted credentials and reconnect cookies are excluded from diagnostics/persistence. This internal service handover is independent of the Linux user credential at GDM.

Actual authorized Fedora 44 integration renders GDM after service authentication and handover. Twelve real resolution changes and three new-transport reconnects pass, with synthetic local GTK controls on an isolated compositor. No Linux password or keyboard input was sent; the probe disabled clipboard. Linux desktop sign-in, external GNOME text/image clipboard, physical input and automatic second-stage login remain unverified. Existing assisted field delivery still requires explicit selected-tab/endpoint/login-screen/focus confirmation. Remote GNOME/GDM configuration was unchanged.

Upstream origin: IronRDP commit `e4143f26306a6639a889d80c1c484ca69c365a8f` for EGFX/graphics, registry session 0.11.0 and connector 0.10.0 at `11a0810cfbbabd8b8023875a05e3041216d4b01b`; MIT OR Apache-2.0, with retained notices and per-file provenance. Compatibility adapters preserve the existing core/PDU APIs. AVC420/444 are disabled; no H.264 capability is advertised. Existing RSA advisory remains visible and unfixed. Qualified Fedora RPM/signing and physical-desktop acceptance remain open.

The full product is not yet complete. The stable source lane contains operational compiled code and exact regression evidence; it does not qualify the entire product or a Fedora binary distribution. Authorized Windows authentication/framebuffer decoding passes; Fedora GNOME service authentication, RDSTLS handover and actual GDM rendering are verified; Linux-user sign-in remains separate. Physical input, dependency security and the remaining terminal, files, agents, automation and distribution scope remain open.

| Requirement | Current evidence | Status / remaining work |
|---|---|---|
| D-00 independent windows | Actual GTK/shared-daemon window close and independent private layouts | PASS bounded native fixture; first window owns default GUI automation endpoint; RDP transport transfer between processes remains |
| RDP-06 GNOME dual login | Separate credentials, assisted input and opt-in recognized native login | Real authorized Fedora automatic Linux sign-in PASS; tab/missing-secret refusal PASS. Other layouts/locales, background tabs and wrong real GDM password remain open |
| T-01 Screen commands | Nine real-PTY copy/chooser/prompt/configuration and 2–5 RUWAC nesting groups | PARTIAL: implemented bounded headless interaction; mixed GNU Screen/tmux 2–5 layers pass in scoped configurations; Screen arbitrary-control paste and physical input qualification remain |
| T-02 startup input | 100 adversarial probes in headless suite | PASS fixture |
| T-03 multiple PTYs and load | Independent PTYs, layout suite, actual 128-pane process/wire/native paging fixture | PASS bounded capacity; maximum-load terminal/checkpoint performance remains separate |
| T-04 detach and reattach | Native crash/reattach and workspace same-PID tests | PASS daemon lifetime; no process resurrection claim |
| T-05 stable IDs | Targeted routes, stale-ID negatives, concurrent clients | PASS |
| T-06 VT/Unicode/applications | Native Vim/less/htop and actual fullscreen fzf Unicode/Alt navigation, canonical extended SGR/plain+palette recovery and native/Cairo output | PARTIAL: complete VT/application/Kitty/AltGr/IME and physical-input matrix |
| T-07 mouse recovery | Native routing and disposable Bash raw-child exit | PARTIAL: physical touchpad gate blocked |
| T-08 history/internal buffer | Canonical history, paged capture, independent internal/system clipboard tests | PASS bounded fixture |
| T-09 SSH/nested terminals | Real authorized SSH remote-PTY detach/reattach and Unicode | PARTIAL: actual 2–5 mixed GNU Screen/tmux/RUWAC layers pass with both prefix modes; Screen arbitrary-control paste remains an upstream limitation |
| T-10 performance | Retained matched 1/5/20/40-pane measurements | PARTIAL: current 30-trial matched measurements and per-block unrelated-load accounting implemented; observed desktop/IDE load blocks idle qualification |
| A-01 negotiation/discovery | Existing v0 schema/capabilities/typed errors | PARTIAL: describe/local GUI targeting and typed RDP lifecycle over actual SSH implemented; read-only paged host inventory/list/query now passes native and authenticated SSH tests; workspace open/restore remain |
| A-02 literal input | Unicode, quotes, shell metacharacters and separate Enter | PASS |
| A-03 concurrent routes | Idempotency, focus changes, stale IDs | PASS |
| A-04 non-TTY/SSH | Real --host JSON/file control and three SSH PTY cycles | PASS authorized SSH fixture; endpoint provisioning remains explicit |
| A-05 capture/wait/events | Bounded capture, event subscription, revision and wait tests | PASS |
| A-06 socket/backpressure | Owned daemon socket, malformed frames, concurrency limits | PARTIAL: same-UID peer verification and byte-bounded process metadata/all-page native guards validated; broader response bounds and fuzz/soak remain |
| A-07 GUI control | Explicit unavailable response and independent headless graph | PARTIAL: independent permission-scoped focus/RDP lifecycle over actual SSH validated; GUI events and remote input remain |
| R-01 Windows | Authorized Windows 11 authentication and real 1280×720 framebuffer decoding | PASS bounded real-host login/frame; external clipboard, display/input and broader interoperability remain |
| R-02 Fedora GNOME | Actual authorized service login, greeter/user-desktop handovers, automatic Linux sign-in and dynamic viewport | PARTIAL: supported English real-host path PASS; physical input, other locales/layouts and external clipboard remain open |
| R-03 independent tabs | Three authenticated loopback tabs; 200 switches/450 inactive offers | PARTIAL: external-host qualification |
| R-04 display modes | Actual Display Control/reactivation, viewport transforms and resize storm | PARTIAL: external server and physical pointer acceptance |
| R-05 remote keys | Scancode mapping and send-key menu | PARTIAL: actual Windows/GNOME layouts, AltGr/IME/compositor restrictions |
| R-06 pointer/touchpad | Synthetic routing, held-input release and reconnect | PARTIAL: physical input and external hosts |
| R-07 HiDPI | Fractional transforms and actual 3K lossless bitmap frames | PARTIAL: real monitor moves/scales and external codec |
| R-08 responsiveness | Frame timings, cadence and process samples in loopback | PARTIAL: matched authorized Remmina/FreeRDP comparison |
| C-01/C-02 text interoperability | 60 bidirectional loopback transfers | BLOCKED: trust/credential profiles configured; external text/image acceptance remains open; Fedora GDM rendering is now implemented |
| C-03/C-04 PNG interoperability | 18 loopback PNG/DIBV5 transfers | BLOCKED: trust/credential profiles configured; external text/image acceptance remains open; Fedora GDM rendering is now implemented |
| C-05 ownership | Real GDK peer and three loopback RDP tabs; Files isolation | PASS owned compositor |
| C-06 PRIMARY/internal separation | Independent internal/CLIPBOARD tests | PARTIAL: PRIMARY protocol and physical middle-click acceptance |
| C-07 application paste | Real GDK peer, native applications, 10 KiB/1 MiB payloads | PASS owned compositor |
| C-08 local agent image | Safe owned PNG staging | OPEN: supported agent must actually acknowledge image consumption |
| C-09 remote agent image | SFTP integrity transport is available | OPEN: destination staging, receipt and separate-host consumer acceptance |
| C-10 transactions/errors | Epoch cancellation, policies, reconnect and limits | PASS bounded fixture; further malformed/slow-server soak remains |
| C-11 unsupported format/consumer | Visible staging errors and no fake attachment claim | PARTIAL: final destination/consumer fallback UX |
| F-01 import | Explicit read-only Remmina preview, 70 imports, metadata backup | PARTIAL: explicit-ID reviewed merge, policy preservation, stale/active guards and Undo validated; broader nonsecret mapping/skip reporting remains |
| F-02 library | Native compact sidebar/panels, CRUD, groups, favorites, recents, 1,000-profile fixture | PARTIAL: native 50-row paging and all 1,000 endpoints/search validated; transactional canonical SQLite inventory and legacy recovery validated; inherited routes remain |
| F-03 transfers | SFTP SHA-256, native pause/resume/discard, original-host binding, lost-rename recovery and encrypted keys through Secret Service; 14 new and six existing fixture groups | PARTIAL: native rsync file/folder copies, actual authorized SSH integrity and cancellation pass; client SIGKILL recovery and native/external verified backup handling pass; partial-data resume, broader implementations, remaining independent-actor races and remote power-loss durability remain |
| F-04 agents | Ordinary terminal execution, bounded process metadata, native search/exact jump and private D-Bus exit delivery | PARTIAL: observed process layer validated; supported agent image consumption, physical notification acceptance and optional structured adapters remain |
| F-05 ordinary user/headless package | Independent graphical-free headless graph | PARTIAL: per-user installation implemented; RPM dependency/install qualification remains open |
| F-06 install/upgrade/uninstall | Reversible private per-user installer and desktop entry | PARTIAL: per-user installer fixtures pass; Fedora RPM lifecycle remains open |
| F-07 standard workflow | Existing build/release/controller and verified tagged source releases | PARTIAL: immutable binary/RPM publication remains unqualified |
| F-08 public hygiene | Explicit permitted source inventory, outgoing-history checks | PASS for verified source publications |
| Mixed workspace | Live mixed GTK trees and all-view checkpoints/crash restore | PASS bounded native fixture: persistent tab/view IDs, exact leaf focus, move/close and crash restoration; external-host/physical qualification remains separate |
| Persistent metadata/history after daemon loss | Real SIGKILL and runtime-tree removal restore stable IDs, canonical active-screen tail/cells, layout and ended status; native copy passes | PARTIAL: 128 dense 100 by 200 dual-buffer PTYs checkpoint below the unchanged limit and preserve recorded cell styles/text after SIGKILL under a verified 1536 MiB service cap; full-scrollback saturation, arbitrary entropy and idle latency qualification remain |
| Native accessibility and diagnostics | GTK controls, configurable keys and accessible visible RDP toolbar | PARTIAL: global Alt-number navigation and safe targeted doctor implemented; physical keyboard/accessibility matrix open |
| Distribution/governance | Locked dependency license inventory and source release receipts | PARTIAL: Apache-2.0 and contributor/security/conduct/CODEOWNERS/templates implemented; pinned Fedora format/Clippy/unit/real headless/declaration checks and guarded labels added. BLOCKED desktop RSA advisory; hosted/live-label/branch-rule, broader nightly/fuzz/hardware CI and qualified package/signing/provenance remain |

Native endpoint-bound certificate review/pinning and changed/stale guards have four policy units and eight authenticated local native groups. Real Windows authentication/framebuffer decoding passes with an existing approved leaf; the Fedora SSH-attested leaf is verified and GNOME service authentication/handover/GDM rendering passes. Owner clipboard/keyring/session data are never synthetic fixtures; authorized external credential lookup is read-only. Optional browser/agent adapters and later macOS builds remain staged scope.


Authorized Fedora SSH headless/real remote-PTY acceptance passed on 2026-10-11. Mixed terminal/RDP/Files checkpoint restoration and normal application activation are implemented; detailed remaining view-management gates apply. Every remaining full-product gate above continues to apply.


Owner requires automatic GNOME account selection and sign-in on Connect. The supported English Fedora GDM path now passes on a real authorized host. Universal/unattended background login remains unqualified; assisted delivery remains available for unsupported screens.
