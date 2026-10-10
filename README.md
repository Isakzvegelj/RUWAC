# RUWAC

A bounded Rust terminal multiplexer prototype: one daemon owns PTYs; replaceable TUI and JSON clients control sessions, windows and stable panes.

Implemented: persistent daemon-owned children, two clients with shared focus, a horizontal split, literal input and separate Enter, bounded grid capture, an independent text buffer, event/wait APIs, private Unix IPC and local PNG staging. Desktop clipboard/PRIMARY, image attachment, graphics, full Screen compatibility and recovery after daemon loss remain open.

Build with Rust/Cargo in your chosen build workspace. Keep build output and generated evidence outside the source tree:

```sh
export CARGO_TARGET_DIR="$(mktemp -d)"
cargo build --locked --release --manifest-path ruwac/Cargo.toml
cargo test --locked --workspace --manifest-path ruwac/Cargo.toml
export EVIDENCE_DIR="$(mktemp -d)"
PYTHONDONTWRITEBYTECODE=1 python3 ruwac/tests/integration/spike.py \
  --binary "$CARGO_TARGET_DIR/release/ruwac" --evidence "$EVIDENCE_DIR"
```

Start the daemon in one terminal:

```sh
"$CARGO_TARGET_DIR/release/ruwac" serve
```

In another terminal, use the same build directory:

```sh
R="$CARGO_TARGET_DIR/release/ruwac"
"$R" rpc session.create --params '{"name":"development"}'
# Use the returned session and pane IDs:
"$R" attach --session session_ID
"$R" rpc pane.sendText --params '{"pane":"pane_ID","text":"literal $HOME and ž"}'
"$R" rpc pane.sendKey --params '{"pane":"pane_ID","key":"Enter"}'
"$R" rpc pane.capture --params '{"pane":"pane_ID","lines":250}'
"$R" version
```

The default socket is beneath your private runtime directory. `RUWAC_SOCKET` or `--socket PATH` selects an explicit socket; its parent must be owned by you with mode 0700. Use the same socket in each terminal.

Ctrl-A then `c/n/p/d/a` creates/cycles/detaches/sends literal Ctrl-A. Ctrl-A `S` splits; `Tab` changes pane; `0–9` selects windows. `[` copies a bounded capture internally, `]` pastes it; interactive selection is pending. PTYs survive final-client detach. `rpc daemon.stop` stops the selected daemon and its owned panes.

Unix PTYs are required. The tested toolchain is Fedora Rust 1.98.1. Cargo.lock pins registry checksums. Prereleases describe a prototype, not production or desktop acceptance. Tagged source prereleases preserve operational implementation and engineering history. The product distribution license remains undecided.

See [architecture decisions](architecture/00-RUWAC-DECISION.md), [protocol](architecture/02-PROTOCOL-V0.md), [acceptance](architecture/03-ACCEPTANCE-MATRIX.md) and [benchmarks](architecture/04-BENCHMARKS.md).

## Native engineering prerelease

The retained headless core has named target-specific commands and an optional compiled Rust GTK4/libadwaita client, with real GDK text clipboard transport and private PNG staging. See [the implementation and evidence report](spec/RUWAC_INTEGRATION_SPIKE.md), [ADR-001](spec/RUWAC_ARCHITECTURE_ADR.md), and [acceptance limits](spec/RUWAC_ACCEPTANCE_STATUS.md). RDP, PRIMARY acceptance, image attachment, host library, files and RPM packaging are not completed. This milestone is distributed as a source prerelease with the acceptance limits listed below.


The desktop is a separate Cargo workspace. Build it with `cargo build --locked --release --manifest-path ruwac/desktop/Cargo.toml`, using the same external CARGO_TARGET_DIR as the core. `ruwac gui` starts the sibling `ruwac-desktop`; Python GI is retained as a test oracle. Native dependencies are optional, and neither workspace embeds an RDP backend. See the current engineering reports before treating fixture success as desktop acceptance.
