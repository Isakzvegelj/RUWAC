# RUWAC local API — native client integration

Round `v3-native-20261010`, 2026-10-10. The implementation was validated as a local engineering candidate. Subsequent owner authorization requires tagged source prerelease publication through the project build/release workflow; runtime promotion and RPM qualification remain separate. Original candidate reports are preserved privately in build results.

Protocol remains version 0, u32 big-endian length plus UTF-8 JSON, 1 MiB maximum frame. Explicit IDs and correlation values are preserved. Headless socket is owned, 0700 parent/0600 socket, SO_PEERCRED same-owner authenticated. No TCP listener or new desktop service endpoint.

Native worker queries schema/capabilities and uses session.attach, client.render, client.input, client.resize and session.detach on persistent authenticated connections. Canonical snapshots add `mouse_sgr`, `alternate_screen` and `application_cursor` mode flags; these are additive fields without a protocol version change. Stale revisions are rejected. GUI-only workspace/RDP commands still return GUI_UNAVAILABLE: no dummy desktop IPC success.

Existing named create/list/split/send-text/send-key/capture/wait and raw events subscription remain compatible. CLI `gui` now dispatches the sibling compiled `ruwac-desktop`, with an optional explicit executable override; it no longer defaults to the Python GI oracle. The CLI itself has no GTK dependencies.

`send-text --file` remains UTF-8 only, 256 KiB per request, rejects NUL, performs no shell expansion and adds no implicit Enter. Desktop text has a separate 1 MiB cap, split only at UTF-8 boundaries into 32 KiB acknowledged input messages. An application-requested paste wraps every chunk separately. A partial failure/cancellation cannot undo already delivered bytes; the visible error reports acknowledged byte count. Shell newline execution depends on receiving application/readline policy; raw receiver tests do not promise safe shell commands.

Image data never uses `send-text` or an enlarged JSON frame. GDK `image/png` streams into private files; image-stage validates/hash-copies with 64 MiB encoded/decoded and 4096² dimension caps. Paths are same-owner local fixture transport, not arbitrary remote capabilities. Symlinks, invalid PNGs, excessive sizes/dimensions and unsafe staging directories fail. This is staging, not attachment.

Portable candidate usage (from the build binding):

```sh
target/release/ruwac --socket "$SOCKET" serve
target/release/ruwac --socket "$SOCKET" session create --name work --detach --json
target/release/ruwac --socket "$SOCKET" gui --session session_ID
```
