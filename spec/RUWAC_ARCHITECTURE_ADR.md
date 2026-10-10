# ADR — persistent native layout and embedded IronRDP

Desktop interaction and embedded RDP increment, 2026-10-10. Earlier reports are retained privately; this report supersedes their current-state claims.
Accepted for this implementation. Keep daemon-owned PTYs and canonical terminal parsing. Store a binary split tree, ratios and zoom in each window; render rectangles from canonical cell coordinates. Limit each window to 16 panes. Closing a pane collapses its branch. Layout survives graphical-client detach and reconnect; daemon persistence across daemon failure is not implemented.

Provide paged, bounded canonical scrollback with generation checks rather than returning an unbounded RPC payload. Copy selection belongs to GTK TextView; internal paste remains authenticated daemon state. Approximate memory budgeting counts cells and does not include every combining-character allocation or metadata byte.

Preserve one worker per terminal view and bounded/coalesced responses. Close drains queued input before detach. Reset terminal input modes on foreground-group changes and EOF. Emit actual SGR or legacy X10 bytes in the daemon according to canonical mode; Shift gestures override application reporting.

Use original RUWAC integration with IronRDP 0.17.0 and ironrdp-tokio 0.10.0 in the optional desktop workspace. Network/TLS/NLA/decode execute on a worker; GTK receives the latest bounded BgrX framebuffer. System trust and optional explicit CA validate certificates and hostnames. Avoid the permissive example TLS upgrader. Remote deactivation requests fail visibly and require reconnect. No external viewer, nested window or copied GPL component is used.

Physical GNOME keyboard/touchpad/PRIMARY acceptance is unqualified. Weston does not provide PRIMARY. Real authenticated RDP requires an explicitly authorized endpoint, username, Secret Service profile and trusted server certificate/CA; none is designated. RDP CLIPRDR, dynamic remote resizing, clipboard image attachment acknowledgement, IME, accessible terminal semantics and RPM distribution remain unfinished. TUI copy remains bounded capture rather than the desktop interactive copy UI. Foreground/EOF terminal mouse and alternate-screen state reset is implemented; restoration of termios after an abruptly killed child is not certified. Daemon or host failure still loses in-memory sessions.

The read-only copy TextView unregisters its implicit PRIMARY selection clipboard on realization. Internal keyboard mark/search therefore cannot silently take compositor PRIMARY. Desktop selection remains an explicit broker operation. Search uses editable change notifications rather than delayed search-entry events.
