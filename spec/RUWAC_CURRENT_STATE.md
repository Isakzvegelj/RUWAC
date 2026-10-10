# RUWAC current state — native Rust candidate

Round `v3-native-20261010`, 2026-10-10. The implementation was validated as a local engineering candidate. Subsequent owner authorization requires tagged source prerelease publication through the project build/release workflow; runtime promotion and RPM qualification remain separate. Original candidate reports are preserved privately in build results.

Canonical source retains the owner-renamed uppercase RUWAC binding; build/Git authority retains lowercase ruwac. Exact detected paths, owner, leases, source hashes and dirty inventory are in `results/v3-native/preflight.json`. Branch `main`, HEAD `7abbf537d1f608b8555a027bd9e97d172d937e22`. Existing staged/index changes remain byte-for-byte unchanged; new source edits remain unstaged. No second repository, daemon replacement or directory rename.

| Component | Current state |
|---|---|
| Persistent headless daemon / PTY / canonical grid | retained, optimized regression PASS |
| Compiled Rust GTK4/libadwaita frontend | implemented, separate optional Cargo workspace |
| Desktop text clipboard | real cross-process GDK Wayland copy/read/paste PASS with generated fixtures |
| PRIMARY | implemented through GDK; test compositor lacks protocol, BLOCKED |
| Internal Screen buffer | independent daemon state, bounded 256 KiB |
| PNG clipboard staging | real MIME stream into owned files, 64 MiB encoded/decoded caps, hash and cleanup PASS |
| Native input/selection | tested subset; physical gestures and complete Screen map remain partial |
| RDP / library / agent attachment / RPM | not integrated; real-host acceptance BLOCKED |

Fedora 44; Rust 1.98.1; GTK 4.22.5; libadwaita 1.9.4; Weston 15.0.1. Native Rust uses gtk4 0.11.5, libadwaita 0.9.2 and GLib/Pango 0.22. Headless Cargo dependencies and ELF linkage exclude GTK/adwaita/VTE/RDP/WebKit/Secret Service. The GTK process owns widgets; the daemon remains sole parser and PTY owner.

Owner cmux PID 62168 remained running. All test daemons and sockets were owned disposable fixtures, then stopped. The normal default RUWAC socket was not stopped or replaced. No owner clipboard payload was read. Live GNOME tests excluded clipboard operations.

The installed 0rig exposes its own release workflow and version controller, but no qualified S015 app-linux handler was found. Local supervisor read-only check returned `local-check-failed`; no instructions were executed. Development used local installed contracts and the supplied handoff, without a new Drive fetch.
