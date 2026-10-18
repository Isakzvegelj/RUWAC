# RUWAC Remmina sign-in, terminal copying and SSH image delivery

Reviewed Remmina imports can copy saved service passwords from Secret Service,
including missing passwords for matching existing endpoints. Existing RUWAC
passwords remain unchanged. GNOME desktop passwords remain separate. Self-signed
RDP servers prompt for certificate review on the selected foreground tab; automatic sign-in retains the useful
connection failure instead of hiding it behind a generic error.

Terminal mouse selection preserves soft wraps as one command and strips grid
padding before real line breaks, preserving shell continuation backslashes.
Mouse selection in history exports PRIMARY for middle-click paste into other apps. Selection highlights clear when the selected screen text or selection owner changes. Current engines supply wrap metadata directly; compatible live older engines use verified matching history without restarting their programs.
The updater diagnoses a missing GitHub CLI verifier before downloading an archive.

New saved SSH and agent launches retain a private control socket owned by the
original pane's SSH process. Implicit PNG uploads open a channel on that same
connection, even after OpenSSH aliases or jump configuration change. A missing,
closed or replaced transport refuses upload without contacting an alternate host.
Older terminals remain open; reopen their bookmark for protected image delivery.

The remote receiver verifies the declared bytes and hash, then waits for local
publication authorization. Cancellation or timeout before that commit discards the
temporary file while preserving the terminal's connection. Already committed cache
images retain the documented interrupted-delivery limits.

Source installation accepts an explicit --target-dir and public instructions use
Cargo directly, without private maintainer adapters. Default source layouts remain
supported; testing bundles reject overrides of their verified artifact paths.
README navigation now matches the current Map keys default and separates SSH
Terminal from Files; current-state summaries distinguish historical notes.

Tests use generated local SSH servers, keys, homes and payloads. They do not prove
physical-input, every external provider or host, seamless RDP frontend handoff,
or full distribution/security qualification. Existing engines and active frontend
connections are preserved during this increment.
