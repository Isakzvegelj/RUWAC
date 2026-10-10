# Reuse decision — retain the measured RUWAC foundation

Round `v3-native-20261010`, 2026-10-10. The implementation was validated as a local engineering candidate. Subsequent owner authorization requires tagged source prerelease publication through the project build/release workflow; runtime promotion and RPM qualification remain separate. Original candidate reports are preserved privately in build results.

No competitor replacement, fetch or repeated broad evaluation was performed. Existing RustConn pin `760b0c4747a26f511fdac93d5aaea1e47bd8201f` was inspected only for the proposed tab/lifecycle boundary and GTK binding alignment. Workspace declaration is GPL-3.0-or-later. Exact manifest/license/tab-container/tab-lifecycle/child-teardown hashes are in `results/v3-native/provenance.json`.

No RustConn or cmux source file was copied. All new desktop modules are original RUWAC implementation using documented GTK/libadwaita APIs. The file-level audit therefore has zero imported competitor files and does not claim GPL clearance for a future port. RustConn's independent PTY spawn/relay/child teardown remains excluded because RUWAC daemon owns process lifetime.

GTK 0.11.5/adwaita 0.9.2 compiled first as a minimal target, then as the working client. GTK 0.9 reference types are not mixed into the binary. Locked GUI dependency inventory and ELF links are retained in private receipts. Distribution license/notice review remains open; local build success is not license clearance for an RPM.

The former cmux GTK protocol mismatch remains a historical reference result. No unsupported protocol translation was introduced. Typed synthetic RDP views do not count as embedded RustConn/IronRDP integration or real RDP acceptance.
