# Native Rust implementation and evidence — v3-native-20261010

Round `v3-native-20261010`, 2026-10-10. The implementation was validated as a local engineering candidate. Subsequent owner authorization requires tagged source prerelease publication through the project build/release workflow; runtime promotion and RPM qualification remain separate. Original candidate reports are preserved privately in build results.

Implemented `ruwac/desktop` with compiled Rust GTK4/libadwaita widgets, Adwaita reorderable tabs, canonical Pango/Cairo terminal cells, focused panes, native key/drag/click/wheel controllers, resize, bounded asynchronous workers and detach-preserving close. Python GI remains only the old oracle and independent generated clipboard peer. CLI gui starts the compiled executable.

Implemented GDK CLIPBOARD and PRIMARY request paths, actual Unicode text streams, independent internal Screen buffer, metadata transfer tickets with focus/generation cancellation, disabled transport policy and image/png staging. Large PNG fixture transfers outside JSON; 64 MiB encoded/decoded allocation limits, 4096² dimensions, safe file modes, SHA-256 and cancellation/cleanup were exercised. The Stage clipboard PNG button is an explicit staging action, not agent delivery.

Canonical selection is copied through a separate GDK consumer. Terminal Shift+drag selects instead of reporting mouse; normal clicks/wheel report SGR only when requested and focused, with pane-relative coordinates and daemon mode recheck. Legacy X10/UTF-8 mouse, hover-only motion modes, modifier-rich mouse, full AltGr/IME/accessibility and complete Screen compatibility remain unqualified. Existing bounded history capture is searchable in a native text view; selected history can be copied to the internal buffer with Enter. Physical drag, full history-search interaction and RDP key routing are not accepted by synthetic tests alone.

Native callback cards passed c/n/p/0, pane Tab and literal nested Ctrl+A. Numeric 0–9 and window listing use daemon discovery; horizontal S uses existing two-pane limits. Vertical | and : command prompt remain explicit gaps; help reports them. No second parser or PTY lifecycle was imported.

Real Weston native fixture: two session tabs and three panes, 200 switches, key-handler bytes, SGR enable/disable, 1 MiB paste, delayed IPC/backpressure, clipboard MIME/size/hash, PNG/negative caps/cancellation and close/reattach. A separate GDK process supplies/consumes generated payloads. A valid synthetic input serial is needed for compositor clipboard ownership; fake-seat diagnostics are not accepted as cross-process proof. PRIMARY failed because the Weston registry exposes no primary-selection manager. Owned live GNOME smoke passed the previously stalled tab-switch case with synthetic callbacks; clipboard operations were excluded.

Regression: 15 unit tests, all 11 real-daemon groups including 100 startup races, named CLI and isolated Vim/less/htop. Headless dependencies remain graphical-free. Performance script retested current and preserved previous candidates against tmux, 30 samples per action/count; host noise disclosed in acceptance report.

Exact optimized SHA-256:

- headless: `e65701ddaa7ae7e694b0c34c4945530d2389d5fb050cf14fb72a3e4392bf9c56`
- desktop: `77361f9a1d1e3baac69c650f1995bcf30bf7da43072b1d3ac3384bcdbee4ff03`

Private local entrypoints:

```sh
./build --release --test --desktop
./build --debug --desktop
./build.sh --help
./release.sh --help
```

`RUWAC_CARGO_HOME` is recorded in the private command receipt for this node. Compilers run only in the registered build binding through the existing source bridge. GTK/GLib system development headers are required only for desktop builds.

Candidate bundle: `artifacts/ruwac-v3-native`; launch `run serve` in one terminal and `run gui` in another. Use Ctrl+A S for a horizontal split. `run --version` verifies the retained package identity. Closing GUI keeps the demo daemon/shells alive; `run rpc daemon.stop` ends only that bundle's demo socket. No stable pointer or installation is modified. This is a local development bundle, not an RPM, release or complete desktop application.

Logs, source manifests, exact paths/tool versions/dependency graphs, previous immutable receipts, synthetic PNGs and all pass/fail diagnostics remain under build results/evidence. Current receipt links each executed check to its tested binaries. Abrupt shutdown cleanup, full gesture/keyboard cards, native application-mode GUI cards and physical PRIMARY/browser/editor persistence remain the next input qualification work.
