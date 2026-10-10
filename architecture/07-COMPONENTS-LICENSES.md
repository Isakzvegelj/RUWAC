# Components and license boundaries

Direct crates are pinned in Cargo.toml; Cargo.lock pins transitives and registry checksums. An inventory generated from locked Cargo metadata records dependency versions, declared licenses and repository URLs in `dependency-licenses.json`.

| Component | Version | Declared license | Use |
| --- | --- | --- | --- |
| portable-pty | 0.9.0 | MIT | PTY and child lifecycle |
| alacritty_terminal | 0.26.0 | Apache-2.0 | canonical terminal grid |
| Tokio | 1.53.1 | MIT | bounded actor and Unix IPC |
| serde / serde_json | 1.0.228 / 1.0.151 | MIT OR Apache-2.0 | JSON |
| libc | 0.2.189 | MIT OR Apache-2.0 | Unix permissions/signals/termios |
| uuid | 1.26.1 | Apache-2.0 OR MIT | opaque IDs |
| sha2 | 0.10.9 | MIT OR Apache-2.0 | PNG receipt |
| png | 0.18.0 | MIT OR Apache-2.0 | PNG validation |

Verify licenses from the downloaded registry manifests and preserve dependency notices before binary distribution. No source from ttmux, cmux, Ghostty or either existing zmux repository is incorporated. cmux's Cargo license is not assumed to override its root/cloud terms. This document records declared component terms, not legal clearance or a license grant for newly authored ruwac code.

Primary references: [Alacritty source](https://github.com/alacritty/alacritty), [portable-pty source](https://github.com/wezterm/wezterm/tree/main/pty), [Tokio](https://github.com/tokio-rs/tokio), [vte](https://github.com/alacritty/vte).
