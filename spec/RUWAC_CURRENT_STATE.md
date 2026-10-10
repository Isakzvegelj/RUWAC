# RUWAC current state

Desktop interaction and embedded RDP increment, 2026-10-10. Earlier reports are retained privately; this report supersedes their current-state claims.
The existing daemon and canonical alacritty terminal engine remain the foundation. The CLI/headless workspace has no GTK or RDP dependencies. The separate compiled GTK4/libadwaita desktop now has Screen window navigation, native chooser/command/help dialogs, interactive keyboard selection and search, internal paste, two split orientations, divider ratio updates, zoom and daemon-owned layout persistence.

Scrollback defaults to 50,000 lines, capped by configurable approximate cell-storage budget. History is paged over protocol v0; stale history epochs fail visibly. Copy view is bounded to 8 MiB, with a visible truncation warning. Internal Screen buffer stays separate from GDK CLIPBOARD and PRIMARY.

A genuine embedded IronRDP client is implemented in the same Adwaita TabView: asynchronous connection/NLA, certificate and hostname validation, Secret Service reference, decoded native framebuffer, scaling, pointer/key routing, send-key actions and disconnect/reconnect. Local TLS rejection/trust tests execute real protocol negotiation and TLS. No real remote login has been performed.

Physical GNOME keyboard/touchpad/PRIMARY acceptance is unqualified. Weston does not provide PRIMARY. Real authenticated RDP requires an explicitly authorized endpoint, username, Secret Service profile and trusted server certificate/CA; none is designated. RDP CLIPRDR, dynamic remote resizing, clipboard image attachment acknowledgement, IME, accessible terminal semantics and RPM distribution remain unfinished. TUI copy remains bounded capture rather than the desktop interactive copy UI. Foreground/EOF terminal mouse and alternate-screen state reset is implemented; restoration of termios after an abruptly killed child is not certified. Daemon or host failure still loses in-memory sessions.
