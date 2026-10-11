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

Verify licenses from the downloaded registry manifests and preserve dependency notices before binary distribution. No source from ttmux, cmux, Ghostty or either existing zmux repository is incorporated. cmux's Cargo license is not assumed to override its root/cloud terms. This document records declared component terms, not legal clearance for dependency redistribution. RUWAC’s original source is licensed under Apache-2.0, as approved by the owner; the full terms are in LICENSE.

Primary references: [Alacritty source](https://github.com/alacritty/alacritty), [portable-pty source](https://github.com/wezterm/wezterm/tree/main/pty), [Tokio](https://github.com/tokio-rs/tokio), [vte](https://github.com/alacritty/vte).

The optional desktop workspace now includes IronRDP 0.17.0 and ironrdp-tokio 0.10.0 (MIT OR Apache-2.0), native-tls 0.2.18 (MIT OR Apache-2.0), tokio-native-tls 0.3.1 (MIT), and x509-cert 0.2.5 (MIT OR Apache-2.0). The expanded locked Linux metadata inventory includes transitive dependencies. RDP glue is original RUWAC code; RustConn GPL patterns were reviewed but no source was copied. [Official IronRDP](https://github.com/Devolutions/IronRDP) is the API provenance reference. Dependency declarations are not packaged-distribution clearance.

The clipboard increment additionally reuses ironrdp-cliprdr 0.7.0 and ironrdp-cliprdr-format 0.2.0. ironrdp-server 0.13.0 is confined to development/test fixtures. These components declare MIT OR Apache-2.0; see the refreshed locked inventory for transitive declarations. Dependency licenses are preserved separately from the Apache-2.0 license covering RUWAC’s original source. Binary packaging still requires dependency notice validation.

SFTP uses ssh2 0.9.6 (MIT OR Apache-2.0) over libssh2 (BSD-3-Clause), confined to the desktop workspace. Secret Service uses the installed secret-tool/GIO/libsecret provider. Display Control remains ironrdp-displaycontrol 0.8.0 with a local bounded header adapter. No shared infrastructure or alternate engine was introduced. The owner selected Apache-2.0 for original source; dependency notices and binary packaging remain separate qualification gates.
