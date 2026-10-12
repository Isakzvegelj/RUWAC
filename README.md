# RUWAC

A Rust remote workspace with persistent terminals: one daemon owns PTYs; replaceable TUI and JSON clients control sessions, windows and stable panes.

Implemented: persistent daemon-owned children, shared focus, nested horizontal/vertical splits, zoom, paged scrollback, private Unix IPC and a separate compiled GTK desktop. The desktop supports keyboard marking/search, real GDK clipboard transport, private PNG staging and an embedded IronRDP client. Physical desktop/PRIMARY and authenticated remote-host acceptance remain unqualified; image attachment remains open; private ended-history recovery after daemon loss is implemented with the limits below.

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

Ctrl-A then `c/n/p/d/a` creates/cycles/detaches/sends literal Ctrl-A. Double Ctrl-A switches to the previous window on a matching daemon. Ctrl-A `S` splits horizontally and `|` vertically; `Tab` changes pane; `0–9` selects windows. `[` opens interactive canonical keyboard copy/search in both clients; `]` pastes the independent internal buffer. `z` zooms, `"` opens the window chooser, `:` opens commands and `?` shows configured shortcuts. PTYs survive final-client detach. `rpc daemon.stop` stops the selected daemon and its owned panes.

Unix PTYs are required. The tested toolchain is Fedora Rust 1.98.1. Cargo.lock pins registry checksums. Prereleases describe a prototype, not production or desktop acceptance. Tagged source prereleases preserve operational implementation and engineering history. RUWAC’s original source is licensed under Apache-2.0; see [LICENSE](LICENSE). Dependencies retain their own licenses. Binary packaging remains subject to dependency notice and packaging validation.

See [architecture decisions](architecture/00-RUWAC-DECISION.md), [protocol](architecture/02-PROTOCOL-V0.md), [acceptance](architecture/03-ACCEPTANCE-MATRIX.md) and [benchmarks](architecture/04-BENCHMARKS.md).

## Native engineering prerelease

The retained headless core has named target-specific commands and an optional compiled Rust GTK4/libadwaita client, with real GDK text clipboard transport and private PNG staging. See [the implementation and evidence report](spec/RUWAC_INTEGRATION_SPIKE.md), [ADR-001](spec/RUWAC_ARCHITECTURE_ADR.md), and [acceptance limits](spec/RUWAC_ACCEPTANCE_STATUS.md). External-host RDP, physical PRIMARY acceptance, image attachment and RPM packaging remain open. The first machine sidebar and authenticated loopback RDP/CLIPRDR integration are implemented. This milestone is distributed as a source prerelease with the acceptance limits listed below.


The desktop is a separate Cargo workspace. Build it with `cargo build --locked --release --manifest-path ruwac/desktop/Cargo.toml`, using the same external CARGO_TARGET_DIR as the core. `ruwac gui` starts the sibling `ruwac-desktop`; Python GI is retained as a test oracle. Native dependencies are optional; only the desktop workspace includes IronRDP. See the current engineering reports before treating fixture success as desktop acceptance.

The desktop inventory uses system SQLite through rusqlite. Fedora desktop builds also require `sqlite-devel`; the installed desktop requires the SQLite runtime library. The headless daemon and CLI require neither. Private `machines.sqlite3` is canonical storage; `machines.json` remains a compatibility bridge for older windows. Conflicting or malformed storage is preserved and unsafe writes are refused. Closing old editors before concurrent edits avoids races with older releases that do not participate in SQLite transactions.

Release versions use numeric `major.minor.patch`, with odd minor numbers for beta and even minor numbers for stable. Repeated releases in the same lane advance the patch. Historical suffix tags are retained as earlier numbering mistakes; subsequent releases follow the numeric policy.


Launch the desktop from the same external build directory and socket:

```sh
"$CARGO_TARGET_DIR/release/ruwac" gui --session session_ID
# Repeated --session IDs on ruwac-desktop open additional persistent tabs.
```

Create windows with Ctrl-A c, split with Ctrl-A S or |, move between panes with Ctrl-A Tab, and zoom/restore with Ctrl-A z. Drag dividers to change persistent ratios. Ctrl-A [ opens canonical history: arrows/hjkl, Page Up/Down, g/G, Space to mark, Enter to save internally, / to search and Escape to return. Ctrl-A ] pastes that buffer. Ctrl-Shift-C/V use desktop CLIPBOARD separately. Ctrl-A d detaches; opening the same session again retains running programs and layout.

Private configuration may set RUWAC_SCROLLBACK_LINES (default 50000) and RUWAC_SCROLLBACK_MIB (default 256; approximate cell budget). Copy view has an 8 MiB cap with visible truncation. RUWAC_SHORTCUTS remaps GTK accelerators for prefix/copy/paste; RUWAC_KEYMAP remaps prefix commands. Both remapping variables contain JSON objects, for example `RUWAC_SHORTCUTS='{"prefix":"<Control>b"}'` and `RUWAC_KEYMAP='{"x":"z"}'` (x invokes zoom). The TUI supports Control prefixes and prefix-command remapping. `v` toggles rectangular selection, `w/b/e` move by Unicode words, and `n/N` repeat searches. Oversized selections and cancellation preserve the existing internal buffer. See client help for the current configuration.

“New RDP tab” opens the in-process client. Supply an authorized host:port, username, a Secret Service profile reference and optional trusted CA PEM. The lookup uses attributes application=RUWAC and profile=the reference; secrets are not stored in the project. System certificate/hostname validation is mandatory. Fit/native/stretch/fullscreen and explicit send-key actions are implemented. Text and PNG CLIPRDR works in both directions with five clipboard policies, explicit send/receive and remote paste. Authenticated loopback sessions are tested; dynamic resizing is qualified against authenticated loopback sessions; external Windows/GNOME hosts remain unqualified.

“Machines” toggles the sidebar; Ctrl+K searches names, full groups, endpoints and tags. Favorites and recent-use filters are available. Previous/next controls show 50 profiles per page; search and filter changes start at the first matching page. Choose a Remmina file explicitly to preview approved nonsecret metadata, then import it. Passwords and key/certificate files are never imported. To update an existing endpoint, preview the file, explicitly select its endpoint identity, and use Review merge into selected endpoint. Review the field changes before Apply; local IDs, credential references, trust/key paths and preferences are preserved. Active endpoints and changes made after review require closing tabs or reviewing again. Cancel leaves metadata unchanged; Undo restores the previous private library. RDP entries open native tabs; SSH entries create daemon-owned SSH terminals. Native profile editing and Set/replace/forget credential controls are implemented.

## Machine workspaces and Files

Use Machines → New machine / endpoint or Network groups / defaults, and Edit beside an endpoint. Add endpoint retains the machine identity; duplicate creates new identities and credential references. Set/replace/forget credentials use system Secret Service. Metadata Undo restores the previous private library. RDP connects inside its tab; SSH opens a persistent terminal plus a native Files tab. Files verifies known_hosts before authentication and offers navigation, upload/download, explicit overwrite and progress. Pause transfer retains a private journal and partial; Pending transfers… resumes or explicitly discards the selected job. Resume verifies the original host, source and retained prefix before continuing and verifies the complete staged file before commit. Original files remain protected during overwrite and interrupted rename recovery. For encrypted SSH keys, configure the key file and the separate SSH key passphrase reference, then use Set / replace key passphrase; only the Secret Service reference is saved. Transfers currently handle individual regular files up to 32 GiB, with 128 pending jobs; recursive transfer and rsync remain open. Dynamic RDP uses negotiated Display Control; fit/native/stretch, zoom, fullscreen and Pan viewport remain available. See the current acceptance report for fixture evidence and remaining defects.

## Install and open the workspace

Use the existing build workflow after a controller-assigned release:

```sh
./build.sh --release --desktop --install-user
ruwac gui
```

The per-user installer adds a RUWAC desktop launcher and commands in the standard user binary directory. `ruwac gui` starts or reuses the persistent daemon. Use New terminal for another session and Sessions to reopen a running one. Closing a terminal tab detaches it; automatic checkpoints and normal window close save terminal/RDP/Files tabs, split geometry and selected tab. RDP restores disconnected, and Files restores its directory without automatic authentication. `ruwac start` starts the daemon without a graphical client.

The full headless suite and three genuine SSH remote-PTY detach/reattach cycles also pass on an authorized Fedora host. This does not establish external RDP login or physical-desktop acceptance. See [all remaining product gates](spec/RUWAC_PRODUCT_COMPLETION.md).

Use **Split tabs…** to place existing terminal, RDP or Files views side by side or above/below each other. **Separate views** returns them to tabs while preserving connections. RDP reconnect/disconnect, fit, dynamic resolution, zoom and fullscreen are visible in the native toolbar. Additional connection, remote-key and clipboard actions are under **RDP controls**. Clipboard ownership follows the focused canvas; local controls and Files cannot let inactive RDP sessions take over CLIPBOARD.

**Views…** lets you move one mixed view to its own tab or close it. Closing a terminal view detaches it; use Sessions to reopen its still-running programs. Closing RDP disconnects just that view. Closing Files pauses its transfer; reopen the same endpoint’s Files tab to resume or discard the retained job. Neighboring views and connections remain open. Empty workspaces can create a new terminal from the toolbar.

## Control another host

Use an existing SSH alias/key and verified host key. The endpoint must have RUWAC installed:

```sh
ruwac --host SSH_HOST describe
ruwac --host SSH_HOST start
ruwac --host SSH_HOST session list --json
ruwac --host SSH_HOST pane send-text --id pane_ID --file local-input.txt --json
ruwac --host SSH_HOST pane send-key --id pane_ID --key Enter --json
ruwac --host SSH_HOST attach --session session_ID
```

Global `--remote-binary PATH` and `--socket REMOTE_PATH` select an explicit executable/socket. Remote request content is literal JSON over stdin. Password prompting, agent forwarding and desktop forwarding are disabled. `describe` exposes actual endpoint capabilities and supports older live daemons through read-only aggregation; it does not upgrade or restart them.

## Desktop navigation and diagnostics

Launch `ruwac gui`. Alt+1 through Alt+9 select workspace tabs 1–9; Alt+0 selects tab 10. The shortcuts work from terminal, RDP and Files views. Missing tabs are a no-op. Ctrl+Alt and Shift+Alt remain available to applications. Use `ruwac gui --theme system` to follow the desktop color preference, or `--theme light` / `--theme dark` explicitly.

`ruwac doctor --desktop` checks local runtime availability without changing it. `ruwac doctor --terminal` inspects the current terminal. After a crashed client leaves raw/no-echo attributes, run `ruwac doctor --terminal --repair` directly in that affected foreground terminal. Repair never targets another terminal and does not reset emulator mouse/private modes. Remote read-only checks use `ruwac --host SSH_HOST doctor`; remote terminal repair is refused.

## Address native workspace views

Run the desktop, then query its separate local API using the same daemon/socket target:

```sh
ruwac workspace describe --json
ruwac workspace list --json
ruwac workspace activate --id "$RUWAC_TAB_ID" --json
ruwac workspace focus --id "$RUWAC_VIEW_ID" --json
```

Use actual IDs returned by list; titles and numeric positions are not automation identities. Enable **Allow automation to change workspace focus** in the desktop menu before activate/focus. The permission resets to off for a new launch; `ruwac gui --allow-workspace-control` enables it for that launch. Listing needs a running GUI and never launches one. Clipboard contents and RDP input are outside this API. `input_focus_view` reports actual GTK input focus separately from remembered focus, which may briefly precede widget mapping. Background focus changes do not grant remote clipboard ownership.

The native **Views… → Focus** action selects one mixed-workspace view. IDs and each tab’s selected view persist in schema-three metadata. Older workspace files get a private migration backup; terminal programs continue running across desktop restarts.

## Recover history after daemon loss

The ordinary daemon checkpoints private metadata and a bounded canonical history tail under the standard user state directory. Restart restores the same session/window/pane IDs as ended history, without starting their old commands. Open Sessions to view it, use the existing copy controls, and choose New terminal to start a live session.

`ruwac rpc daemon.checkpoint --params '{}'` waits for a completed checkpoint. `ruwac rpc health --params '{}'` reports checkpoint progress/failures. Changed state saves every two seconds; a crash may lose changes since the last completed save. The retained tail is up to 50,000 lines or 512 KiB per pane, with explicit truncation counts. Internal clipboard buffers, input/request logs and credentials are not serialized; displayed output remains private terminal state.

Default state is `RUWAC/terminal-history.json` under `XDG_STATE_HOME` or the standard home state directory. Custom sockets keep a sibling history file; set `RUWAC_STATE_FILE` to a file beneath an existing owned 0700 durable directory when persistence across runtime-directory removal is needed. Files must be owned regular 0600 files; corrupt/unsafe archives are preserved and rejected. Closing ended panes and checkpointing removes their history. Hidden primary-screen history while an alternate-screen application runs is not yet stored separately. Existing older daemons are preserved during installation and require a later owner-controlled restart to gain checkpoints.

## Native machine and RDP controls

The searchable machine sidebar shows grouped profiles with native protocol icons, names and endpoints. Use **New machine / endpoint…** to create a profile and **Edit → Set/replace credential** to enter its password through Secret Service. Passwords remain out of the profile file and command arguments. A connection still requires trusted server identity.

The RDP toolbar reuses the same authenticated connection and display handlers as the advanced controls. Reconnect/disconnect affects only that view; fit and zoom do not restart the connection. Dynamic resolution requests a negotiated remote display change, with local fit as a fallback when unsupported. Closing the GUI preserves daemon-owned terminal programs. A previously running older GUI remains active during installation; close/reopen its window to see a newly installed interface. Older daemons visibly reject unsupported previous-window navigation without losing subsequent input.

The GitHub workflow publishes verified source releases and does not compile downloadable executables. Local build.sh compiles the independent headless binary and optional GTK binary. A final stable release and Fedora binary/package publication remain subject to the acceptance gates in the product audit.

### Native RDP automation

`ruwac rdp list --json` and `ruwac rdp query --id view_ID --json` read safe attached-view metadata. `ruwac rdp open --profile endpoint_ID --json`, `ruwac rdp reconnect --id view_ID --json` and `ruwac rdp disconnect --id view_ID --json` require the desktop permission “Allow automation to open/control RDP connections”. Open defaults to background operation; optional `--activate` also requires focus permission. Startup is asynchronous: query phase and decoded updates to establish readiness. Disconnect keeps the view; closing makes its ID stale. GUI operations reject idempotency keys.

Prefix these commands with `--host user@host` to use the existing verified SSH transport and installed remote RUWAC CLI. The remote desktop must already be running with its trusted GUI socket and matching permissions. Missing GUI endpoints do not start graphical or daemon processes. This does not enable remote input, credential retrieval, clipboard reads or event subscriptions.

The machine sidebar keeps saved endpoints, search/filter controls, paging and **New machine / endpoint…** visible. Expand **Machine tools** for network defaults, refresh and Undo; expand **Import from Remmina** for explicit file selection, preview and reviewed import/merge. Both panels start collapsed. The import area scrolls when expanded, and diagnostic text is selectable.

### Mixed terminal compatibility

Reproduce the two-to-five-layer GNU Screen/tmux/RUWAC matrix with `python3 ruwac/tests/integration/mixed_nesting.py --binary /path/to/ruwac --evidence /path/to/private-evidence`. The fixture creates only private sessions/sockets/configuration and records actual tool versions, leaf PID/start time, byte hashes and dimensions. GNU Screen and tmux are test-only tools; missing tools mean blocked compatibility qualification.

Both common Control-A and native tmux Control-B configurations are exercised. Escape a prefix at each intermediate layer to address a deeper layer. Normal Unicode/multiline paste and a single bracket pair are verified; tmux/RUWAC also contain control prefixes in paste. GNU Screen 5.0.1 interprets its own Control-A commands inside a pasted payload: do not assume arbitrary-control paste is safe through Screen. Explicit `pane.sendText` to a known leaf RUWAC daemon bypasses intermediate client key parsers. [GNU Screen’s command bindings](https://www.gnu.org/software/screen/manual/screen.html) describe its detach and literal-prefix controls.

Process metadata transport limits: `job.list` and `job list` return at most the requested 100 rows and a 64 KiB serialized metadata page. Follow `next_offset` until null to read the complete collection. The native Jobs… monitor performs this traversal automatically, verifies membership and unique stable IDs, and displays 50 rows per page. It retains the previous display when a collection changes mid-read and retries on the next poll. Full-capacity testing covers 128 actual panes with long Unicode metadata; larger correlation IDs, other response types and broader protocol fuzz/soak remain separate gates.

Contribution and repository checks: see [CONTRIBUTING.md](CONTRIBUTING.md), [SECURITY.md](SECURITY.md) and [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md). The pinned Fedora workflow checks formatting, Clippy, units, real headless fixtures and declared dependency/checksum/graph boundaries on pull requests and main. Separate advisory checks remain failing for the documented unpatched desktop RSA dependency; this is not stable security qualification. Contribution labels use trusted base code and do not merge changes or publish artifacts.


Native windows, the overlay sidebar and GNOME dual-credential storage are documented in [GNOME login and desktop windows](spec/RUWAC_GNOME_LOGIN.md). GNOME modes support the non-AVC Graphics Pipeline and verified same-endpoint RDSTLS handover. Real Fedora GDM rendering is tested; Linux user sign-in remains a separate explicit action.

Fedora 44 updates: use the header's update icon or `ruwac update --check` and `ruwac update --install`. Automatic checks announce newer stable GitHub releases; explicit installation builds published source locally and retains active sessions and the previous version. See [prerequisites, bootstrap and rollback](spec/RUWAC_UPDATES.md). Prebuilt RPM delivery remains open.
