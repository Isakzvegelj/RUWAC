# Acceptance evidence

Current desktop interaction and embedded IronRDP state is recorded in [the acceptance report](../spec/RUWAC_ACCEPTANCE_STATUS.md) and [current architecture decisions](../spec/RUWAC_ARCHITECTURE_ADR.md). Earlier measurements and design limits below are historical evidence.


Canonical real-daemon run: `evidence/spike-4jw0njyz/acceptance.json`; full unit/build log: `evidence/build-test-complete.log`. Seven focused Rust tests passed. All eleven integration groups passed. Earlier harness failure (Python function shadowed a module) is retained and excluded from product failures. Binary hashes and immutable report links are in [evidence-index.json](evidence-index.json); portable receipt copy: [acceptance-results.json](acceptance-results.json).

| Case | Status | Observed proof / limits |
| --- | --- | --- |
| C01 literal CLI + Enter | PASS | JSON stdin request carries dollars, quotes, backslash, tabs, UTF-8, emoji, combining text and newline into raw child; exact hex match; Enter separately adds 0d; NUL/oversize rejected |
| C02 startup races | PASS | 100 real TUI startups, injections at 0/1/2/5 ms; exact cumulative receiver bytes, delayed DA/OSC color replies excluded; 96 injections observed before first canonical grid; no byte loss in any run |
| C03 stable focus + two views + split | PASS | shared selected window visible to both; explicitly targeted original pane receives bytes while another is focused; minimum client size 20×70; two split grids sum to 20 rows; one detach leaves other attached; closed pane gives STALE_ID |
| C04 concurrency + retries | PASS | five concurrent independent clients deliver each complete chunk once, order unspecified; duplicate idempotency key writes once; changed request rejected |
| C05 controlled mouse + paste | PASS | disabled SGR discarded, enabled forwarded exactly, disabled again discarded; bracketed paste containing Ctrl-A d reaches child without detaching; modes reset on exit |
| C06 detach/crash lifecycle | PASS | normal detach, client SIGTERM and SIGKILL preserve daemon-owned child PID; reconnect works; termios/reset bytes checked for normal/SIGTERM; SIGKILL cannot run cleanup |
| C07 socket/frame/start lock | PASS | owned 0600 socket, duplicate daemon rejected; zero/oversize frames close, malformed JSON structured error; daemon's descriptor inodes have no TCP listeners |
| C08 events/wait | PASS | same framing delivers state.changed; pane.wait wakes on captured marker, absent marker returns TIMEOUT |
| C09 local image stage | PASS | synthetic validated 2×2 PNG privately staged 0600 with exact bytes/SHA-256; invalid MIME rejected; owned staged file cleaned; attached=false |
| C10 shutdown without reader | PASS | shutdown client closes before ack; daemon exits cleanly rather than leaving a listener over a stopped core |
| C11 real Screen-prefix navigation | PASS | TUI Ctrl-A c/n/p/d creates/cycles/detaches; a and doubled Ctrl-A deliver exact 0101; child PID survives |
| Grid state | PASS | unit checks truecolor, wide glyph, combining accent, emoji, mouse/paste modes, alternate screen and application capability reply; OSC 52 ignored |
| Input framing | PASS | fragmented UTF-8/CSI, DA/OSC replies, paste prefix isolation and oversized-paste drain unit tests |
| Internal A / external B | PARTIAL | daemon internal buffer remains independent of a synthetic external variable; no real desktop clipboard accessed/tested |
| Full Screen keymap/copy selection | PARTIAL | basic prefix slice implemented; bounded capture copy differs from selection UI; remaining keys explicitly open |
| Vim/less/htop/fzf visual compatibility | UNTESTED | Vim/less/htop available, fzf absent; no interactive visual acceptance performed |
| Cross-UID negative test | UNTESTED | same-UID peer policy implemented; socket-mode and owned-listener tests do not replace a second UID test |
| Nested mux, prolonged soak, output saturation | UNTESTED | requires additional owned fixtures and duration; no pass inferred from library tests |
| CLIPBOARD/PRIMARY/outer-emulator shortcuts | BLOCKED | needs authorized graphical workstation and recorded outer emulator |
| Physical mouse/touchpad and drag transitions | BLOCKED | synthetic protocol injection cannot prove physical routing or root cause |
| Actual application image attachment / graphics | BLOCKED | local PNG staging is not attachment or rendering |
| Second-host SSH/remote PNG/lifecycle | BLOCKED | no evaluation host designated; GitHub authentication is unrelated to this gate |
| API performance | PASS | 30 samples at 1/5/20/40 panes, all 12 median ratios within 2× tmux; idle qualification and raw samples in benchmark report |
| Keystroke-to-render | PASS | separate 30-sample instrumented TUI measurement; not physical photon or real visual judgment |
| Historical evidence preservation | PASS | all 4,960 manifest entries verified unchanged after relocation to evaluation |

**Decision: GO for the next bounded TUI reliability milestone.** Core correctness and the measured API target support continued experimentation. Production deployment, production binary distribution, full multiplexer replacement and desktop/image claims remain NO-GO until their separate gates pass. If a hard correctness gate regresses or performance misses its target, profile and reconsider tmux/frontend or ttmux repair before expansion.
