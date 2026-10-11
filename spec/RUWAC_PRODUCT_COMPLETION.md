# RUWAC product completion audit

The full product is not yet complete. A runnable source prerelease and isolated fixture passes are evidence of implementation, not proof of Windows/Fedora interoperability or production desktop acceptance. This audit retains the full scope: persistent terminal, native mixed workspace, remote desktops, unified clipboard, SSH/files, agent cockpit, automation and Fedora distribution.

| Requirement | Current evidence | Status / remaining work |
|---|---|---|
| T-01 Screen commands | Native Screen card and headless prefix tests | PARTIAL: full headless interactive copy/chooser and nested behavior need qualification |
| T-02 startup input | 100 adversarial probes in headless suite | PASS fixture |
| T-03 multiple PTYs and load | Independent PTYs, layout suite, prior 40-pane API fixture | PASS bounded fixture |
| T-04 detach and reattach | Native crash/reattach and workspace same-PID tests | PASS daemon lifetime; no process resurrection claim |
| T-05 stable IDs | Targeted routes, stale-ID negatives, concurrent clients | PASS |
| T-06 VT/Unicode/applications | Native Vim/less/htop, Unicode, alternate-screen and input tests | PARTIAL: complete fzf/combining/extended-input matrix |
| T-07 mouse recovery | Native routing and disposable Bash raw-child exit | PARTIAL: physical touchpad gate blocked |
| T-08 history/internal buffer | Canonical history, paged capture, independent internal/system clipboard tests | PASS bounded fixture |
| T-09 SSH/nested terminals | Real authorized SSH remote-PTY detach/reattach and Unicode | PARTIAL: full 2–5-level nesting |
| T-10 performance | Retained matched 1/5/20/40-pane measurements | PARTIAL: current idle-qualified 30-trial acceptance |
| A-01 negotiation/discovery | Existing v0 schema/capabilities/typed errors | PARTIAL: missing describe/host/workspace API surfaces |
| A-02 literal input | Unicode, quotes, shell metacharacters and separate Enter | PASS |
| A-03 concurrent routes | Idempotency, focus changes, stale IDs | PASS |
| A-04 non-TTY/SSH | Real SSH CLI/PTY round trips | PARTIAL: explicit host option |
| A-05 capture/wait/events | Bounded capture, event subscription, revision and wait tests | PASS |
| A-06 socket/backpressure | Owned daemon socket, malformed frames, concurrency limits | PARTIAL: same-UID peer verification implemented; further fuzz/soak |
| A-07 GUI control | Explicit unavailable response and independent headless graph | PARTIAL: usable permission-scoped GUI/RDP automation adapter |
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
| F-01 import | Explicit read-only Remmina preview, 70 imports, metadata backup | PARTIAL: merge only by explicit identity and detailed conflict acceptance |
| F-02 library | Native CRUD, groups, favorites, recents, 1,000-profile fixture | PARTIAL: inherited route execution and practical large-library pagination |
| F-03 transfers | SFTP SHA-256, overwrite, cancel, slow-peer/loss and native listing | PARTIAL: recursive/resumable/rsync, encrypted keys and cleanup after loss |
| F-04 agents | Ordinary daemon-owned terminal execution | PARTIAL: process metadata, monitor/jump/notify; optional structured adapters |
| F-05 ordinary user/headless package | Independent graphical-free headless graph | PARTIAL: per-user installation implemented; RPM dependency/install qualification remains open |
| F-06 install/upgrade/uninstall | Reversible private per-user installer and desktop entry | PARTIAL: per-user installer fixtures pass; Fedora RPM lifecycle remains open |
| F-07 standard workflow | Existing build/release/controller and verified tagged source releases | PARTIAL: immutable binary/RPM publication remains unqualified |
| F-08 public hygiene | Explicit permitted source inventory, outgoing-history checks | PASS for verified source publications |
| Mixed workspace | Live mixed GTK trees and all-view checkpoints/crash restore | PARTIAL: individual mixed view move/close now qualified; stable GUI view IDs and exact leaf focus remain open |
| Persistent metadata/history after daemon loss | PTYs persist across client loss | OPEN: restore history/metadata after daemon restart with ended state |
| Native accessibility and diagnostics | GTK controls and configurable keys | PARTIAL: keyboard coverage, accessibility matrix and targeted doctor |
| Distribution/governance | Locked dependency license inventory and source release receipts | PARTIAL: Apache-2.0 approved; package qualification, contributor/security docs and broader CI |

External-host and physical-seat gates remain separate from implementation. No owner clipboard/keyring/session may be used as an acceptance fixture. Optional browser/structured agent adapters and later macOS builds remain staged scope; no protocol or scheduler replacement is justified by those features.


Authorized Fedora SSH headless/real remote-PTY acceptance passed on 2026-10-11. Mixed terminal/RDP/Files checkpoint restoration and normal application activation are implemented; detailed remaining view-management gates apply. Every remaining full-product gate above continues to apply.
