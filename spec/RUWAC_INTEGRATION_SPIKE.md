# RUWAC integration evidence — authenticated RDP and clipboard

2026-10-11 increment from release v0.1.2, commit 1633b8265240f32d1f7629c851c729f2bcb4889c. The existing engine, CLI, native Screen interaction, source/build bindings and Git history are retained. This report concerns implemented code and executed tests.

## Implementation

The optional GTK workspace registers IronRDP CLIPRDR and uses the library's initialization/capabilities, format-list response, format-data request/response and conversion components. A connection worker owns TCP/TLS/NLA/decoding and bounded input commands. GUI completion publishes a GDK provider only after FocusBroker rechecks the active tab, owner epoch, policy and remote generation. Inactive offers carry only metadata and never auto-promote on later focus. Provider identity suppresses echo announcements. Binary data stays outside daemon JSON.

Unicode text is normalized between UTF-16LE/CRLF and UTF-8; the GDK provider explicitly publishes text/plain;charset=utf-8. A real independent peer exposed the initial unspecified-MIME conversion defect, which was corrected before passing the text cards. PNG/DIBV5 retains alpha; classic DIB carries RGB. Conversion and decoding run off GTK. Limits are 1 MiB UTF-8 text, 64 MiB image data/decoded allocation and 4096 per image dimension. SVC fragments are bounded before reassembly; the UI queue allows 16 events and 64 MiB aggregate data. One request/decode is allowed per view. Five-second response deadlines cancel safely; remote timeout disconnects to avoid correlating a delayed response with a later request.

RDP tabs use mandatory system TLS/hostname validation or an explicit trusted CA. Passwords come from Secret Service references in normal use; the test path is restricted to generated localhost fixtures. Transient stream/TCP failures get at most three retries with 0.5/1/2-second backoff. Certificate/authentication failures are not retried automatically. Disconnect clears frames and stale clipboard state; modifier/button releases get a bounded best-effort flush before the socket closes. Fit/native/stretch/fullscreen remain available. Controls wrap within the GTK window.

The sidebar implements Ctrl+K search, favorites, recents, full group paths and tags. Explicit Remmina preview/import leaves the source file untouched and excludes passwords and key/certificate paths. Nonsecret schema storage is bounded, private and atomically replaced. FIFO/symlink paths are rejected. RDP endpoints open native tabs and SSH endpoints create daemon-owned PTYs through the existing authenticated protocol, with port/username handling outside GTK.

## Executed evidence

Optimized build/test: 18 headless Rust units, 11 integration groups with 100 startup-input races, three real-daemon layout/history/mouse groups and 10 desktop units. Desktop units include complete authenticated loopback sessions, wrong-password rejection, TLS trust rejection, pointer transforms, Unicode bounds, PNG/DIB/DIBV5 pixel conversion, SVC caps, crash-time staging and 70-profile parsing/FIFO rejection.

The native terminal suite executed 21 PASS cards and one explicit PRIMARY blocker on owned Weston/Xvfb. It exercised actual Vim/less/htop, Unicode file save, keyboard selection/search/internal paste, layout/zoom, independent GDK peer data, caps/cancellation, delayed IPC and crash cleanup. Synthetic input is labelled; the applications and GDK transport are real.

The new RDP suite executed seven PASS groups: three authenticated native tabs; 60 Unicode text transfers; 18 PNG/DIBV5 transfers with decoded pixel comparisons; five policies; server disconnect/reconnect and clipboard afterward; 200 ownership switches with 450 inactive offers; and 70 native GTK profile imports/search/favorite. The RDP server is ironrdp-server 0.13.0, bound exclusively to localhost, using generated credentials/CA and generated 320×180 frames. This is actual CredSSP/NLA, wire traffic, server encoding and client decoding, rather than simulated framebuffer injection or TLS-only negotiation. It is not a Windows or GNOME remote-desktop host.

First decoded-frame observation was 328.2, 277.4, 276.0 ms for the three sessions. This includes connection/authentication and fixture polling; it is not network RTT or keyboard-to-photon latency. Reconnect observation, including the follow-up clipboard card, was 1395.8 ms. The GTK viewport was 800×374 inside a 1054-pixel-wide window. Sampled RSS after three RDP tabs, a terminal and profile cards: GTK 153.7 MiB, fixture server 32.2 MiB. These are process samples, not idle-qualified memory/FPS guarantees.

The owned raw-child regression killed only a verified child in a disposable Bash PTY. Bash restored ECHO/ICANON and exact Unicode input; canonical mouse/SGR reset prevented escaped mouse bytes leaking into the shell. Bash correctly re-enables its own bracketed-paste mode; the earlier assertion expecting it to disappear was corrected and retained as a failed receipt. No general termios reset was introduced into live applications.

## Matched CLI benchmark

30 process-start-inclusive CLI samples per cell, alternating RUWAC and isolated tmux 3.7c, matched receiver workload at 1/5/20/40 panes across windows. Six-vCPU Fedora test VM; CPU busy fraction about 54.3% before and 54.3% after. The host was not idle-qualified. All 12 median ratios met the existing 2× criterion.

| Panes | Action | RUWAC p50 / p95 ms | tmux p50 / p95 ms | Median ratio |
|---|---|---|---|---|
| 1 | list | 2.439 / 5.043 | 4.421 / 8.158 | 0.55 |
| 1 | send | 3.498 / 8.152 | 5.019 / 11.874 | 0.70 |
| 1 | capture | 2.519 / 5.164 | 4.280 / 9.092 | 0.59 |
| 5 | list | 3.497 / 7.757 | 4.676 / 10.295 | 0.75 |
| 5 | send | 2.219 / 7.246 | 3.715 / 8.873 | 0.60 |
| 5 | capture | 2.573 / 5.619 | 4.746 / 9.557 | 0.54 |
| 20 | list | 2.213 / 4.652 | 4.293 / 10.157 | 0.52 |
| 20 | send | 2.488 / 8.614 | 4.366 / 7.779 | 0.57 |
| 20 | capture | 2.054 / 8.275 | 3.727 / 10.310 | 0.55 |
| 40 | list | 3.355 / 7.734 | 5.000 / 9.120 | 0.67 |
| 40 | send | 2.801 / 5.103 | 4.981 / 9.499 | 0.56 |
| 40 | capture | 2.346 / 6.266 | 4.811 / 9.507 | 0.49 |

TTY key-to-emitted-render observation: p50 36.49 ms, p95 42.74 ms, 5 ms polling granularity. This is not physical photon latency. At 40 panes, sampled daemon RSS was 13.76 MiB and TUI RSS 5.28 MiB, excluding children.

## Reproducible commands and receipts

Use the existing node build/release adapters and external build directory. Native tests require an owned Weston/Xvfb runtime and private session bus, GDK Wayland, RUWAC_XTEST_DISPLAY for that Xvfb, OpenSSL and Python GI/Pillow. The RDP script creates its own generated certificates, server processes and sockets; it never configures host RDP/PAM/firewall/accounts.

```sh
./build.sh --release --test --desktop
cargo build --locked --release --manifest-path ruwac/desktop/Cargo.toml --example rdp-fixture
python3 ruwac/tests/integration/native_rust.py --binary "$CARGO_TARGET_DIR/release/ruwac" --gui "$CARGO_TARGET_DIR/release/ruwac-desktop" --evidence "$RUWAC_EVIDENCE_DIR" --primary-unavailable
python3 ruwac/tests/integration/rdp_clipboard.py --binary "$CARGO_TARGET_DIR/release/ruwac" --gui "$CARGO_TARGET_DIR/release/ruwac-desktop" --server "$CARGO_TARGET_DIR/release/examples/rdp-fixture" --evidence "$RUWAC_EVIDENCE_DIR"
python3 ruwac/tests/integration/abrupt_child.py --binary "$CARGO_TARGET_DIR/release/ruwac" --evidence "$RUWAC_EVIDENCE_DIR"
python3 ruwac/tests/benchmarks/api.py --binary "$CARGO_TARGET_DIR/release/ruwac" --evidence "$RUWAC_EVIDENCE_DIR"
./release.sh beta
```

Pre-publication optimized executable hashes for the native acceptance run:

| Artifact | SHA-256 |
|---|---|
| Headless CLI | c220bd4cb965cffd21dffe2e0395d30d384a37f02229ceb52a6b538b20e29685 |
| Native desktop | 6daa45d98982a1a35f26b70d42cc519c34147f796bb284200c539eb57b7111bc |
| Disposable RDP fixture | 00ccd7d34f77ffb1bdd90ee5e82cb05b6afae348e68b23b54a271c63007b0101 |

The subsequent special-file preview guard has its own unit gate. Release stamping rebuilds the versioned artifacts; those hashes and the final commit/tag/Release verification are recorded separately in private publication receipts. Public source contains no node build paths or generated artifacts. The existing release controller exclusively allocates the release identity.

Private evidence includes preflight.json, prior-reports, build-final.log, final-unit-gate.log, terminal-native.log, rdp-native-acceptance.json and per-run receipt.json, abrupt-child.json, benchmarks.json, candidate-hashes.json, dependency metadata, exact commands and retained failed runs. Local paths and generated payload bodies remain private. No owner clipboard/library was read, no owner session was stopped and no system service was modified.

## Remaining qualification

1. Qualify an explicitly designated disposable Windows and Fedora/GNOME host. Exercise NLA, certificate handling, remote frames, send-key actions, clipboard consumers and disconnect/reconnect on those hosts. Loopback protocol results do not establish host interoperability.
2. Qualify physical GNOME keyboard layouts/AltGr, PRIMARY, touchpad gestures, divider drag, IME and accessibility on a test seat without capturing private clipboard contents.
3. Extend connection reactivation, negotiated display-control resizing and panning in native-pixel mode. Add deeper cancellation/slow-peer/abrupt-server failure cards. File/HTML CLIPRDR remains outside the supported text/image formats.
4. Add machine/profile creation and editing, endpoint management and Secret Service reference management. Preserve the explicit read-only import preview and full group identity. Qualify SSH against a disposable authorized SSH endpoint.
5. Extend general termios recovery beyond the passing Bash/job-control fixture, copy search navigation, coherent selection under continuous output and TUI interactive selection.
6. Qualify the existing packaging/installation handler and distribution notices before RPM/binary publication. Implement consumer acknowledgement for agent image attachments separately from PNG staging or clipboard transfer.
