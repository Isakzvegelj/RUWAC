# RUWAC source release

This increment adds the native first-connection certificate approval needed for self-signed RDP endpoints. System CA/hostname validation remains the default. The review dialog shows endpoint, validity and full SHA-256 fingerprint, then offers temporary trust for this tab or saved trust for its profile. Exact certificate and host/port binding is checked before authentication; expired, changed and stale approvals fail closed. Changed certificates require explicit typed confirmation. Temporary trust is not saved or checkpointed. There is no generic accept-all TLS option.

Four policy units and eight actual TLS/CredSSP/native dialog groups pass, including authenticated loopback framebuffer, saved and temporary trust, restart, metadata preservation and changed/stale rejection. Existing headless, terminal, clipboard, native profile/workspace, SFTP and SSH regressions repeat on the stamped build. The engine and CLI remain independent of GTK and RDP dependencies.

Actual EVF testing now verifies TLS against its public certificate obtained over the authorized SSH host key. Its GNOME RDP service rejects the supplied account as absent from the SAM authentication database; no external frame/login is claimed. GNOME Remote Login credentials can differ from Linux/SSH credentials. Racun7's certificate was inspected without silently approving it or authenticating. Both remain distinct from authenticated local fixtures. Physical desktop input is still unqualified.

The desktop --new-instance option opens a separate process while preserving existing windows; normal application activation remains unchanged. Read-only RDP metadata reports certificate fingerprint and safe authentication-stage codes without host/CA paths or credential values. Existing permission-scoped control cannot approve certificate trust.

The machine sidebar Save/Edit/Delete no-search fix and visible compiled version remain available. GitHub Fedora quality checks execute separately from the unsuppressed desktop RSA advisory. RUSTSEC-2023-0071 has no qualified upstream fix; no exception is introduced. See spec/RUWAC_DEPENDENCY_SECURITY.md.

Build and publication use existing build.sh/release.sh and release-owned numbering. This is a source prerelease; GitHub publishes source, while matching binaries compile and install locally. Stable security qualification, Fedora RPM/signing/provenance and the remaining full-product acceptance gates remain open. Shared release infrastructure is not modified.
