# zMux

A bounded Rust terminal multiplexer spike: one daemon owns PTYs; replaceable TUI and JSON clients control sessions, windows and stable panes.

Implemented scope: persistent daemon-owned children, two clients with shared focus, a simple horizontal split, literal input and separate Enter, bounded canonical-grid capture, an independent internal text buffer, event/wait APIs, private Unix IPC and local PNG staging. This is a technical experiment. Desktop clipboard/PRIMARY, image attachment, graphics, full Screen compatibility and recovery after daemon loss remain open.

Source lives in `/zDev/zMux`; builds, caches and generated test evidence live under `/app/zMux`. The build script copies an explicit product-file inventory. Private workflow files, original task material and historical binary caches are excluded from Git.

```sh
./build check               # source-only validation
./build release             # compile in /app/zMux
./build test                # unit tests + real daemon/TUI acceptance in /app
mkdir -m 700 /app/zMux/demo
/app/zMux/target/release/zmux --socket /app/zMux/demo/control.sock serve
```

In another terminal:

```sh
Z=/app/zMux/target/release/zmux
$Z --socket /app/zMux/demo/control.sock rpc session.create --params '{"name":"development"}'
# Use the returned session and pane IDs:
$Z --socket /app/zMux/demo/control.sock attach --session session_ID
$Z --socket /app/zMux/demo/control.sock rpc pane.sendText --params '{"pane":"pane_ID","text":"literal $HOME and ž"}'
$Z --socket /app/zMux/demo/control.sock rpc pane.sendKey --params '{"pane":"pane_ID","key":"Enter"}'
$Z --socket /app/zMux/demo/control.sock rpc pane.capture --params '{"pane":"pane_ID","lines":250}'
```

Ctrl-A `c/n/p/d/a` creates/cycles/detaches/sends literal Ctrl-A. Ctrl-A `S` splits; `Tab` changes pane; `0–9` selects windows. `[` copies a bounded capture internally, `]` pastes it; interactive selection is pending. The daemon survives final-client detach. Run `rpc daemon.stop` on this exact socket to end only its owned panes.

Rust/Cargo and a Unix PTY environment are required; the tested toolchain is Fedora Rust 1.98.1. Cargo.lock pins registry checksums. `ZMUX_BUILD_ROOT` may select another directory under `/app`; `ZMUX_CARGO_HOME` selects the build cache. The `/app/zMux` Git checkout can also run `./build release` directly; the configured copy bridge is needed only when syncing from the separate source tree. No installation into system paths or binary release is performed.

See [architecture decisions](architecture/00-ZMUX-DECISION.md), [protocol](architecture/02-PROTOCOL-V0.md), [acceptance](architecture/03-ACCEPTANCE-MATRIX.md) and [benchmarks](architecture/04-BENCHMARKS.md).
