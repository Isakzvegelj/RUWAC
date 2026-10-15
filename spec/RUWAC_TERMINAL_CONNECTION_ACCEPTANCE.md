# Terminal and connection usability acceptance

Baseline: published v0.4.1. This increment preserves the existing daemon protocol and persistent processes.

The CLI keeps Ctrl+A. GTK uses Ctrl+Alt+A for local controls so Ctrl+A reaches applications, including a real nested tmux configured with that prefix. The existing shortcut override remains supported. Both clients support lowercase s for horizontal splitting and vertical bar for vertical splitting. CLI status labels accept SGR mouse clicks on the physical bottom row.

New GUI windows and splits pass the selected login-shell argv explicitly, including against an isolated older daemon. New tabs already used that path. Existing shells retain their original startup flags. Tests use generated shell profiles; actual Bash reads the fixture profile and rc. A live view closes when all session shells have ended; it selects a surviving window first. Reattaching ended history remains possible. Closing a GUI leaves the daemon and its live PTYs running.

GTK uses all allocated rows and up to 200 columns, matching the engine limit. Zero-size/unmapped views wait before attaching rather than temporarily imposing a small default size on shared PTYs. Larger physical terminals and smaller simultaneous attachments retain the documented engine bounds and shared sizing policy.

RDP starts with current Dynamic viewport dimensions or its last decoded size. Automatic reconnect also carries forward the last decoded framebuffer dimensions. Queued input from authentication is discarded before the new active transport processes PDUs. An initial-size-aware authenticated loopback peer verifies three reconnects at 1400×868 with no smaller decoded framebuffer after initial settling. This does not prove external Windows application placement or GNOME interoperability.

Native tab context actions target the RDP view; framebuffer right-click stays remote input. Configured disconnected views expose a centered reconnect overlay. Tab titles omit group/protocol text and status is indicated by icons. Connection status describes observed transport state rather than inferred host availability.

About / Support shows version, Apache-2.0, source and a static QR for https://idz.si/bs/ruwac-donate.html. The dialog uses independently implemented GTK widgets; no extension source was copied. The generated QR was independently decoded. Donations use the existing site payment handoff; automatic invoice integration is deferred.

Validation uses generated profiles, owned PTYs, an isolated older daemon, real tmux and native GTK under an owned Mutter compositor. It is automated input, not physical input. The native terminal suite has 15 groups; shell compatibility has four; CLI usability has nine; terminal exit has three; GTK has 116 unit/integration tests. Privacy regression tests use generated values only. Additional RDP clipboard, trust, package and artifact gates are recorded by the qualified release workflow.

Reproduce with configured build.sh --release --test --desktop and release.sh patch. Public source supports cargo test --locked --workspace and cargo test --locked --manifest-path desktop/Cargo.toml from the Rust workspace, followed by the named Python integration scripts against isolated binaries/compositors. Private release configuration and raw host evidence are excluded from publication.

A running GUI keeps its loaded executable after an update. Installing new files does not disconnect it. Restarting the GUI still replaces its RDP transports; persistent background RDP transport/handoff is not implemented. Physical input, current external Windows/Fedora GDM acceptance, dependency advisories, signed Fedora distribution and Weston popup compatibility remain separate gates. This batch does not claim full product completion.

An initial saved-profile fixture created and closed the editor before its first mapping; that synthetic sequence reproduces a GTK 4.22.5 Wayland input-method crash on baseline v0.4.1 and the candidate. Qualification now waits for an allocated dialog before activating Save & Connect, matching an available user action. The candidate also clears editor focus and defers connection opening until the response unwinds. Four mapped editor/connection cycles pass with input methods enabled. No upstream fix for arbitrary unmapped automation is claimed.

Deep legacy Screen/tmux chains can retain an earlier attach size before their clients finish starting. The nesting fixture records that size and drives an explicit physical resize after the chain accepts input; it then verifies exact propagated sizes, byte integrity and the same leaf process. Immediate attach-time size convergence for arbitrary legacy chains is not claimed.
