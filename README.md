# RUWAC

A bounded Rust terminal multiplexer prototype: one daemon owns PTYs; replaceable TUI and JSON clients control sessions, windows and stable panes.

Implemented: persistent daemon-owned children, shared focus, nested horizontal/vertical splits, zoom, paged scrollback, private Unix IPC and a separate compiled GTK desktop. The desktop supports keyboard marking/search, real GDK clipboard transport, private PNG staging and an embedded IronRDP client. Physical desktop/PRIMARY and authenticated remote-host acceptance remain unqualified; image attachment and recovery after daemon loss remain open.

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

Ctrl-A then `c/n/p/d/a` creates/cycles/detaches/sends literal Ctrl-A. Ctrl-A `S` splits horizontally and `|` vertically; `Tab` changes pane; `0–9` selects windows. `[` opens interactive keyboard copy/search in the desktop (bounded capture in the TUI); `]` pastes the independent internal buffer. Desktop `z` zooms, `"` opens the window chooser, `:` opens commands and `?` shows configured shortcuts. PTYs survive final-client detach. `rpc daemon.stop` stops the selected daemon and its owned panes.

Unix PTYs are required. The tested toolchain is Fedora Rust 1.98.1. Cargo.lock pins registry checksums. Prereleases describe a prototype, not production or desktop acceptance. Tagged source prereleases preserve operational implementation and engineering history. The product distribution license remains undecided.

See [architecture decisions](architecture/00-RUWAC-DECISION.md), [protocol](architecture/02-PROTOCOL-V0.md), [acceptance](architecture/03-ACCEPTANCE-MATRIX.md) and [benchmarks](architecture/04-BENCHMARKS.md).

## Native engineering prerelease

The retained headless core has named target-specific commands and an optional compiled Rust GTK4/libadwaita client, with real GDK text clipboard transport and private PNG staging. See [the implementation and evidence report](spec/RUWAC_INTEGRATION_SPIKE.md), [ADR-001](spec/RUWAC_ARCHITECTURE_ADR.md), and [acceptance limits](spec/RUWAC_ACCEPTANCE_STATUS.md). External-host RDP, physical PRIMARY acceptance, image attachment, file transfer and RPM packaging remain open. The first machine sidebar and authenticated loopback RDP/CLIPRDR integration are implemented. This milestone is distributed as a source prerelease with the acceptance limits listed below.


The desktop is a separate Cargo workspace. Build it with `cargo build --locked --release --manifest-path ruwac/desktop/Cargo.toml`, using the same external CARGO_TARGET_DIR as the core. `ruwac gui` starts the sibling `ruwac-desktop`; Python GI is retained as a test oracle. Native dependencies are optional; only the desktop workspace includes IronRDP. See the current engineering reports before treating fixture success as desktop acceptance.

Release versions use numeric `major.minor.patch`, with odd minor numbers for beta and even minor numbers for stable. Repeated releases in the same lane advance the patch. Historical suffix tags are retained as earlier numbering mistakes; subsequent releases follow the numeric policy.


Launch the desktop from the same external build directory and socket:

```sh
"$CARGO_TARGET_DIR/release/ruwac" gui --session session_ID
# Repeated --session IDs on ruwac-desktop open additional persistent tabs.
```

Create windows with Ctrl-A c, split with Ctrl-A S or |, move between panes with Ctrl-A Tab, and zoom/restore with Ctrl-A z. Drag dividers to change persistent ratios. Ctrl-A [ opens canonical history: arrows/hjkl, Page Up/Down, g/G, Space to mark, Enter to save internally, / to search and Escape to return. Ctrl-A ] pastes that buffer. Ctrl-Shift-C/V use desktop CLIPBOARD separately. Ctrl-A d detaches; opening the same session again retains running programs and layout.

Private configuration may set RUWAC_SCROLLBACK_LINES (default 50000) and RUWAC_SCROLLBACK_MIB (default 256; approximate cell budget). Copy view has an 8 MiB cap with visible truncation. RUWAC_SHORTCUTS remaps GTK accelerators for prefix/copy/paste; RUWAC_KEYMAP remaps prefix commands. Both remapping variables contain JSON objects, for example `RUWAC_SHORTCUTS='{"prefix":"<Control>b"}'` and `RUWAC_KEYMAP='{"x":"z"}'` (x invokes zoom). See native help for the current configuration.

“New RDP tab” opens the in-process client. Supply an authorized host:port, username, a Secret Service profile reference and optional trusted CA PEM. The lookup uses attributes application=RUWAC and profile=the reference; secrets are not stored in the project. System certificate/hostname validation is mandatory. Fit/native/stretch/fullscreen and explicit send-key actions are implemented. Text and PNG CLIPRDR works in both directions with five clipboard policies, explicit send/receive and remote paste. Authenticated loopback sessions are tested; external Windows/GNOME hosts and dynamic remote resizing remain unqualified.

“Machines” toggles the sidebar; Ctrl+K searches names, full groups, endpoints and tags. Favorites and recent-use filters are available. Choose a Remmina file explicitly to preview approved nonsecret metadata, then import it. Passwords and key/certificate files are never imported. RDP entries open native tabs; SSH entries create daemon-owned SSH terminals. A full profile editor and Secret Service management UI remain future work.
