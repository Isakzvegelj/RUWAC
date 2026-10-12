# RUWAC product completion audit

The full product is not yet complete. A runnable source prerelease and isolated fixture passes are evidence of implementation, not proof of Windows/Fedora interoperability or production desktop acceptance. This audit retains the full scope: persistent terminal, native mixed workspace, remote desktops, unified clipboard, SSH/files, agent cockpit, automation and Fedora distribution.

| Requirement | Current evidence | Status / remaining work |
|---|---|---|
| T-01 Screen commands | Nine real-PTY copy/chooser/prompt/configuration and 2–5 RUWAC nesting groups | PARTIAL: implemented bounded headless interaction; mixed GNU Screen/tmux 2–5 layers pass in scoped configurations; Screen arbitrary-control paste and physical input qualification remain |
| T-02 startup input | 100 adversarial probes in headless suite | PASS fixture |
| T-03 multiple PTYs and load | Independent PTYs, layout suite, actual 128-pane process/wire/native paging fixture | PASS bounded capacity; maximum-load terminal/checkpoint performance remains separate |
| T-04 detach and reattach | Native crash/reattach and workspace same-PID tests | PASS daemon lifetime; no process resurrection claim |
| T-05 stable IDs | Targeted routes, stale-ID negatives, concurrent clients | PASS |
| T-06 VT/Unicode/applications | Native Vim/less/htop, Unicode, alternate-screen and input tests | PARTIAL: complete fzf/combining/extended-input matrix |
| T-07 mouse recovery | Native routing and disposable Bash raw-child exit | PARTIAL: physical touchpad gate blocked |
| T-08 history/internal buffer | Canonical history, paged capture, independent internal/system clipboard tests | PASS bounded fixture |
| T-09 SSH/nested terminals | Real authorized SSH remote-PTY detach/reattach and Unicode | PARTIAL: actual 2–5 mixed GNU Screen/tmux/RUWAC layers pass with both prefix modes; Screen arbitrary-control paste remains an upstream limitation |
| T-10 performance | Retained matched 1/5/20/40-pane measurements | PARTIAL: current idle-qualified 30-trial acceptance |
| A-01 negotiation/discovery | Existing v0 schema/capabilities/typed errors | PARTIAL: describe/local GUI targeting and typed RDP lifecycle over actual SSH implemented; host inventory and workspace open/restore remain |
| A-02 literal input | Unicode, quotes, shell metacharacters and separate Enter | PASS |
| A-03 concurrent routes | Idempotency, focus changes, stale IDs | PASS |
| A-04 non-TTY/SSH | Real --host JSON/file control and three SSH PTY cycles | PASS authorized SSH fixture; endpoint provisioning remains explicit |
| A-05 capture/wait/events | Bounded capture, event subscription, revision and wait tests | PASS |
| A-06 socket/backpressure | Owned daemon socket, malformed frames, concurrency limits | PARTIAL: same-UID peer verification and byte-bounded process metadata/all-page native guards validated; broader response bounds and fuzz/soak remain |
| A-07 GUI control | Explicit unavailable response and independent headless graph | PARTIAL: independent permission-scoped focus/RDP lifecycle over actual SSH validated; GUI events and remote input remain |
| R-01 Windows | Authenticated loopback is implemented | BLOCKED: authorized Windows endpoint identified; credential/trust qualification pending |
| R-02 Fedora GNOME | Authenticated loopback is implemented | BLOCKED: authorized GNOME endpoint identified; credential/certificate identity pending |
| R-03 independent tabs | Three authenticated loopback tabs; 200 switches/450 inactive offers | PARTIAL: external-host qualification |
| R-04 display modes | Actual Display Control/reactivation, viewport transforms and resize storm | PARTIAL: external server and physical pointer acceptance |
| R-05 remote keys | Scancode mapping and send-key menu | PARTIAL: actual Windows/GNOME layouts, AltGr/IME/compositor restrictions |
| R-06 pointer/touchpad | Synthetic routing, held-input release and reconnect | PARTIAL: physical input and external hosts |
| R-07 HiDPI | Fractional transforms and actual 3K lossless bitmap frames | PARTIAL: real monitor moves/scales and external codec |
| R-08 responsiveness | Frame timings, cadence and process samples in loopback | PARTIAL: matched authorized Remmina/FreeRDP comparison |
| C-01/C-02 text interoperability | 60 bidirectional loopback transfers | BLOCKED: authorized endpoints identified; RDP credentials/certificate identity pending |
| C-03/C-04 PNG interoperability | 18 loopback PNG/DIBV5 transfers | BLOCKED: authorized endpoints identified; RDP credentials/certificate identity pending |
| C-05 ownership | Real GDK peer and three loopback RDP tabs; Files isolation | PASS owned compositor |
| C-06 PRIMARY/internal separation | Independent internal/CLIPBOARD tests | PARTIAL: PRIMARY protocol and physical middle-click acceptance |
| C-07 application paste | Real GDK peer, native applications, 10 KiB/1 MiB payloads | PASS owned compositor |
| C-08 local agent image | Safe owned PNG staging | OPEN: supported agent must actually acknowledge image consumption |
| C-09 remote agent image | SFTP integrity transport is available | OPEN: destination staging, receipt and separate-host consumer acceptance |
| C-10 transactions/errors | Epoch cancellation, policies, reconnect and limits | PASS bounded fixture; further malformed/slow-server soak remains |
| C-11 unsupported format/consumer | Visible staging errors and no fake attachment claim | PARTIAL: final destination/consumer fallback UX |
| F-01 import | Explicit read-only Remmina preview, 70 imports, metadata backup | PARTIAL: explicit-ID reviewed merge, policy preservation, stale/active guards and Undo validated; broader nonsecret mapping/skip reporting remains |
| F-02 library | Native compact sidebar/panels, CRUD, groups, favorites, recents, 1,000-profile fixture | PARTIAL: native 50-row paging and all 1,000 endpoints/search validated; inherited routes and specified SQLite inventory remain |
| F-03 transfers | SFTP SHA-256, native pause/resume/discard, original-host binding, lost-rename recovery and encrypted keys through Secret Service; 14 new and six existing fixture groups | PARTIAL: recursive/rsync, broader external SSH implementations, independent-actor rename races and remote power-loss durability |
| F-04 agents | Ordinary terminal execution, bounded process metadata, native search/exact jump and private D-Bus exit delivery | PARTIAL: observed process layer validated; supported agent image consumption, physical notification acceptance and optional structured adapters remain |
| F-05 ordinary user/headless package | Independent graphical-free headless graph | PARTIAL: per-user installation implemented; RPM dependency/install qualification remains open |
| F-06 install/upgrade/uninstall | Reversible private per-user installer and desktop entry | PARTIAL: per-user installer fixtures pass; Fedora RPM lifecycle remains open |
| F-07 standard workflow | Existing build/release/controller and verified tagged source releases | PARTIAL: immutable binary/RPM publication remains unqualified |
| F-08 public hygiene | Explicit permitted source inventory, outgoing-history checks | PASS for verified source publications |
| Mixed workspace | Live mixed GTK trees and all-view checkpoints/crash restore | PASS bounded native fixture: persistent tab/view IDs, exact leaf focus, move/close and crash restoration; external-host/physical qualification remains separate |
| Persistent metadata/history after daemon loss | Real SIGKILL and runtime-tree removal restore stable IDs, canonical active-screen tail/cells, layout and ended status; native copy passes | PARTIAL: canonical active and hidden primary buffers survive real SIGKILL and native copy; qualify maximum-load checkpoint performance |
| Native accessibility and diagnostics | GTK controls, configurable keys and accessible visible RDP toolbar | PARTIAL: global Alt-number navigation and safe targeted doctor implemented; physical keyboard/accessibility matrix open |
| Distribution/governance | Locked dependency license inventory and source release receipts | PARTIAL: Apache-2.0 and contributor/security/conduct/CODEOWNERS/templates implemented; pinned Fedora format/Clippy/unit/real headless/declaration checks and guarded labels added. BLOCKED desktop RSA advisory; hosted/live-label/branch-rule, broader nightly/fuzz/hardware CI and qualified package/signing/provenance remain |

Native endpoint-bound certificate review/pinning, volatile tab trust, restart/persistence and changed/stale guards are implemented with four policy units and eight authenticated local native groups. Actual EVF TLS identity is independently verified; its configured GNOME Remote Login account is rejected by the server SAM, and Racun7 awaits certificate approval. Native endpoint-bound certificate review/pinning, volatile tab trust, restart/persistence and changed/stale guards are implemented with four policy units and eight authenticated local native groups. Actual EVF TLS identity is independently verified; its configured GNOME Remote Login account is rejected by the server SAM, and Racun7 awaits certificate approval. External-host and physical-seat gates remain separate from implementation. No owner clipboard/keyring/session may be used as an acceptance fixture. Optional browser/structured agent adapters and later macOS builds remain staged scope; no protocol or scheduler replacement is justified by those features.


Authorized Fedora SSH headless/real remote-PTY acceptance passed on 2026-10-11. Mixed terminal/RDP/Files checkpoint restoration and normal application activation are implemented; detailed remaining view-management gates apply. Every remaining full-product gate above continues to apply.
