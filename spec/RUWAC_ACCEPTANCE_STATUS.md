# RUWAC acceptance — native daily interaction and embedded RDP

Desktop interaction increment, 2026-10-10. Previous reports and failed exploratory receipts remain preserved privately. This table distinguishes working compiled code from physical-desktop and real-host qualification.

| Gate | State | Evidence and limit |
|---|---|---|
| Core regression and security | PASS | 17 units, 11 integration groups, including 100 startup-input races; authenticated private socket, client-close/reattach and PNG caps |
| Persistent layout/history/mouse API | PASS | Three added real-daemon groups: nested layout/ratio/zoom across reconnect, stale history rejection, exact SGR modifiers/X10/motion/disabled bytes |
| Native terminal desktop | PASS fixture | Screen c/n/p/0/Tab/literal Ctrl-A, command errors, split/zoom, keyboard marking/internal paste, 200 exact target switches |
| Native application input | PASS fixture | Real Vim Unicode file save, less last-page navigation and htop exit through compiled GTK handlers |
| GDK text and PNG transport | PASS fixture | Separate GDK peer on isolated Weston, 10 KiB/1 MiB text, PNG 64-square/1440p, caps/cancel/cleanup; no owner clipboard read |
| Crash-time PNG staging | PASS unit | Per-client locks preserve live namespaces and reclaim abandoned owned files; staging does not imply attachment |
| PRIMARY / physical GNOME input | BLOCKED / UNQUALIFIED | Weston lacks PRIMARY; physical gestures/layouts/AltGr not exercised |
| Embedded RDP | PARTIAL | Genuine IronRDP TLS/NLA/decoder integration and same-window native framebuffer compile; synthetic frame drawing and pointer transforms pass |
| RDP TLS trust | PASS local integration | Real loopback RDP negotiation/TLS rejects unknown CA and reaches NLA with explicit trusted CA; mock intentionally ends before login |
| Authenticated remote RDP / performance | BLOCKED | No designated endpoint, username, Secret Service profile and trusted certificate/CA fixture |
| RDP CLIPRDR / dynamic remote resize | UNIMPLEMENTED | Existing focus broker boundary retained; no remote clipboard synchronization claim |
| Abrupt-child termios restoration | UNQUALIFIED | Foreground/EOF resets mouse and alternate-screen state; raw termios recovery is not certified |
| TUI interactive copy | PARTIAL | Existing bounded capture retained; new keyboard selection/search is desktop-only |
| Packaged distribution / runtime promotion | BLOCKED | This increment uses source publication; qualified RPM pipeline not available |

The native fixture is actual compiled GTK4/libadwaita and real cross-process GDK Wayland clipboard transport with synthetic callbacks and generated payloads. It is not physical desktop acceptance. Synthetic embedded RDP rendering does not establish authenticated remote operation. Daemon-owned PTYs survive GUI detach/close; daemon/host failure still loses in-memory sessions.

Scrollback defaults to 50,000 lines, constrained by RUWAC_SCROLLBACK_MIB (approximate cell storage, default 256 MiB). History pages are capped at 500 lines/128 KiB, copy view at 8 MiB with visible truncation, internal buffer at 256 KiB and external desktop text at 1 MiB. Continuous output during paging can yield a visible STALE_HISTORY retry rather than incoherent content. Grid mouse selection across wrapped rows and physical divider gestures require further qualification.

Raw receipts, exact commands, hashes, environment, snapshots and retained failures are private build evidence in the desktop-interaction results. Public source contains no local build paths, prompts or private configuration. Release state is established by the existing release.sh workflow and verified remote refs, rather than inferred from this report.

## API benchmark

Matched RUWAC/tmux receiver workload, 30 CLI samples per cell, alternating order, 1/5/20/40 panes. These are CLI round trips, not physical keyboard-to-photon measurements. The test host was not idle-qualified.

| Panes | Action | RUWAC p50 / p95 ms | tmux p50 / p95 ms | Median ratio |
|---|---|---|---|---|
| 1 | list | 1.617 / 3.049 | 2.866 / 4.878 | 0.56 |
| 1 | send | 1.574 / 3.196 | 2.767 / 4.015 | 0.57 |
| 1 | capture | 1.703 / 2.857 | 2.694 / 5.496 | 0.63 |
| 5 | list | 1.637 / 2.729 | 2.668 / 5.159 | 0.61 |
| 5 | send | 1.644 / 2.919 | 2.889 / 5.290 | 0.57 |
| 5 | capture | 1.598 / 3.634 | 2.736 / 4.942 | 0.58 |
| 20 | list | 1.707 / 2.672 | 2.931 / 5.038 | 0.58 |
| 20 | send | 1.582 / 3.486 | 2.723 / 5.250 | 0.58 |
| 20 | capture | 1.868 / 4.021 | 3.311 / 5.288 | 0.56 |
| 40 | list | 1.841 / 2.461 | 3.422 / 6.621 | 0.54 |
| 40 | send | 1.536 / 2.934 | 2.743 / 4.629 | 0.56 |
| 40 | capture | 1.599 / 3.596 | 2.646 / 4.931 | 0.60 |

At 40 panes, daemon RSS 13.86 MiB and TUI RSS 5.28 MiB, excluding child programs and GTK. No observed median ratio exceeds 2×; this does not certify a quiet-host performance gate. Host CPU busy fractions before/after were 0.296/0.327. Benchmark hashes identify the optimized pre-publication binary; release version stamping does not change these product paths.

## Final compositor qualification

The optimized desktop passes 21 native Weston cards with one PRIMARY blocker (`native-rust-wfswhwr6`), including synthetic native divider gestures, search and a real SIGKILL/restart PNG-reclamation card. GNOME Wayland passes nine rendering/input/lifecycle cards (`native-rust-se6u7slm`) with synthetic callbacks and no desktop clipboard reads. Physical keys/gestures and GNOME clipboard transfer remain unqualified. Internal GtkTextView selection now explicitly unregisters implicit PRIMARY participation; this fixes the reproducible GNOME search-selection failure and keeps internal copy scope independent. Earlier failed receipts are retained.
