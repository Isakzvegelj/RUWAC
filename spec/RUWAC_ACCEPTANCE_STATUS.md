# Acceptance status — compiled native candidate

Round `v3-native-20261010`, 2026-10-10. The implementation was validated as a local engineering candidate. Subsequent owner authorization requires tagged source prerelease publication through the project build/release workflow; runtime promotion and RPM qualification remain separate. Original candidate reports are preserved privately in build results.

| Gate | Result | Executed evidence |
|---|---|---|
| GUI-R1 | PASS | Compiled gtk4 0.11.5/adwaita 0.9.2, two tabs, three daemon-owned panes, native snapshot |
| GUI-R2 | PASS | 200 tab/pane switches through native key handler, exact receiver bytes, no cross-pane input |
| GUI-R3 | PASS | Same three PTY PIDs after close; independent TUI reattach, targeted input and resize |
| GUI-R4 | PASS within fixture | 1 MiB async read/paste, 1-second delayed RPC, saturated bounded queue; UI keeps ticking |
| GUI-R5 | PARTIAL | Native c/n/p/0/Tab/nested Ctrl+A cards PASS; full copy/search/gesture/app cards unqualified |
| INPUT-R1 | PASS synthetic | SGR routing enabled; no disabled-mode leakage; physical touchpad remains untested |
| CLIP-R1 | PASS native integration | Independent GDK peer, canonical and Unicode selection, exact text transfer |
| CLIP-R2 | PARTIAL / BLOCKED PRIMARY | Internal/desktop separation PASS; Weston primary protocol unavailable |
| CLIP-R3 | PASS tested scope | 10 KiB / 1 MiB payloads, delayed stale cancellation, disabled transport |
| IMAGE-R1 | PASS staging | 64²/1440p encoded stream, SHA/dimensions/0600/caps/cancellation/cleanup |
| CORE-R1 | PASS | 15 unit tests, 11 groups, 100 startup input races; named CLI and Vim/less/htop |
| PERF-R1 | PARTIAL | 30 samples/cell at 1/5/20/40 panes; no observed 2× median regression, host not idle-qualified |
| RDP-R0 | PARTIAL | Typed interface and synthetic 3-view focus tests only; no embedded RDP |
| RELEASE-R0 | Numeric source release workflow | Bare numeric tag/package versions; odd minor beta, even minor stable; tested lane semantics through shared controller; allowlisted source and GitHub publication verified separately |

Latest native acceptance: `evidence/native-rust-ethkw0mc/acceptance.json` under the build binding, 17 PASS and one BLOCKED. Owned GNOME smoke: `evidence/native-rust-_s7tzm8p/acceptance.json`, five PASS, synthetic callbacks and no clipboard access. Final core receipt: `evidence/spike-xq84zbzw/acceptance.json`. Full command/log/hash inventory: `results/v3-native/candidate-receipt.json`. Earlier failed seat/focus/PRIMARY diagnostics remain retained. No physical keys, photon latency, real RDP or agent attachment is certified.

The 1 MiB clipboard read/paste pass recorded maximum 16 ms timer gap 49 ms across the fixture; this is main-loop responsiveness, not FPS or keystroke-to-photon latency. Worker queue-full errors remain visible until acknowledged. Image decode/hashing occurs outside GTK; stream writes are bounded 64 KiB local file writes, which are still synchronous disk operations and can delay a frame on unusually slow storage.

## Measured API RTT (milliseconds)

Latest repeat after fixture cleanup, same matched raw receiver workload, alternating RUWAC/tmux order, 30 real CLI process samples per cell. Host aggregate CPU was 28.6% before and 29.7% after; an earlier paired candidate/previous run was 21–31% busy. These are non-idle observations, not performance acceptance under a quiet host.

| Panes | Action | RUWAC p50 / p95 | tmux p50 / p95 | Median ratio |
|---|---|---|---|---|
| 1 | list | 1.607 / 2.492 | 2.671 / 4.818 | 0.60 |
| 1 | send | 1.598 / 3.468 | 2.731 / 4.420 | 0.58 |
| 1 | capture | 1.611 / 3.090 | 2.861 / 4.960 | 0.56 |
| 5 | list | 1.689 / 3.106 | 2.851 / 5.741 | 0.59 |
| 5 | send | 1.548 / 3.251 | 2.599 / 4.336 | 0.60 |
| 5 | capture | 1.662 / 3.410 | 2.766 / 4.847 | 0.60 |
| 20 | list | 1.694 / 2.499 | 2.824 / 4.455 | 0.60 |
| 20 | send | 1.664 / 3.431 | 2.742 / 3.984 | 0.61 |
| 20 | capture | 1.807 / 4.514 | 2.864 / 5.190 | 0.63 |
| 40 | list | 1.752 / 3.289 | 2.833 / 4.370 | 0.62 |
| 40 | send | 1.520 / 3.451 | 2.631 / 4.417 | 0.58 |
| 40 | capture | 1.658 / 2.544 | 2.649 / 4.689 | 0.63 |

At 40 panes: daemon RSS 13.80 MiB, TUI 5.12 MiB, excluding child receivers and GTK. The preserved previous candidate was independently retested with the same script; its 40-pane p50 list/send/capture was 1.990/1.916/1.864 ms under non-idle conditions. None of the observed median ratios exceeded 2×; variation and host activity prevent a certified regression conclusion. Raw paired comparison and the retry are retained in private results.

GTK remains outside headless Cargo and ELF dependencies. System changes this round installed about 2 MiB of Xvfb/xdotool test dependencies; DNF again invalidated a pre-existing pending offline-update transaction. No full upgrade, reboot, owner-session termination or manual update rescheduling occurred. This concrete operational side effect is recorded in `logs/v3-test-input-install.log`.
