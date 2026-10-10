# Desktop clipboard, input and RDP contract — v3-native-20261010

Round `v3-native-20261010`, 2026-10-10. The implementation was validated as a local engineering candidate. Subsequent owner authorization requires tagged source prerelease publication through the project build/release workflow; runtime promotion and RPM qualification remain separate. Original candidate reports are preserved privately in build results.

Actual GDK CLIPBOARD copy/read/paste is connected to FocusBroker. Selection copy uses Ctrl+Shift+C; paste Ctrl+Shift+V. Selection offers PRIMARY and middle click requests PRIMARY when the compositor supports it. Screen history selection/Internal buffer remains independent of both desktop selections. Disabled desktop policy rejects transfers; it never changes the daemon buffer.

| Case | Result | Evidence / boundary |
|---|---|---|
| Canonical terminal selection → independent GDK peer | PASS | Real Wayland data device; synthetic selection range, exact text hash |
| Unicode/multiline selected text copy | PASS | Rust GTK owner → separate Python GDK consumer |
| 10 KiB and 1 MiB external text read/paste | PASS | Separate GDK peer → Rust async stream → raw PTY, exact UTF-8/hash |
| PRIMARY/middle click | BLOCKED on test compositor | Weston 15 does not advertise primary-selection manager; failed attempt retained |
| Internal A / external B independence | PASS | Native fixture and daemon buffer, survives client detach |
| Owner/focus generation and disabled policy | PASS for tested boundaries | Metadata unit tests, delayed native read cancellation and disabled GDK transport |
| 64² and 2560×1440 PNG staging | PASS | Actual image/png stream; 0600, dimensions and SHA-256; large encoded fixture exceeds 8 MiB |
| Cap rejection / invalid PNG / dimension bound / symlink | PASS | Real GDK cap test plus owned file unit tests; rejected outputs removed |
| PNG cancellation / orderly cleanup | PASS | Async read fault delay, no partial/completed staged files after client close |
| Mouse/Screen prefix callbacks | PASS subset / PARTIAL complete map | SGR enabled/disabled test, 200 focus switches, c/n/p/0/a/Tab cards |
| Vim/less/htop | PASS headless | Isolated real application PTYs and TUI; full Rust-GUI app cards remain UNTESTED |
| Native GNOME tab-switch reproduction | PASS owned smoke | Real GdkWaylandDisplay, synthetic callbacks, clipboard excluded |
| Physical GNOME/browser/editor/touchpad/PRIMARY persistence | BLOCKED / UNTESTED | No physical acceptance card executed; no owner clipboard read |
| Typed three-view RDP producer/focus ownership | PASS synthetic / PARTIAL RDP | 200 rounds in unit tests; no embedded backend |
| Windows/Fedora embedded RDP text/images/keys/FPS | BLOCKED | No authorized hosts or implemented backend |
| Agent image attachment/acknowledgement | NOT IMPLEMENTED | Staging is not delivery |

Native fixtures use a disposable Weston Wayland display nested in Xvfb, with XTEST Shift events to acquire valid input serials. This is real GDK inter-process negotiation using synthetic input, not physical desktop acceptance. A seatless/fake-seat Weston run could only see local offers and is retained as a failed diagnostic, never counted as clipboard PASS.

GUI image default cap 64 MiB; staging API permits a smaller configured cap. Text cap 1 MiB; internal buffer/control input 256 KiB; frame 1 MiB. Compressed PNG bytes and decoded pixel allocation are independently limited. No payload text is written to diagnostic logs; only generated fixture files and hash/size metadata are retained privately.

`ruwac/tests/integration/rdp_acceptance.py` records fail-closed cards for three embedded tabs, bidirectional text/PNG, active ownership, modifier keys, 1440p/3K fit/stretch/dynamic/fullscreen, touchpad transforms and disconnect. It performs no network operation and returns blocked without a backend. Real-host selection remains open until explicit connection fixtures exist.
