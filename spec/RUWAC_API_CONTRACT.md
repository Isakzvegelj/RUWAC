# RUWAC local API additions

Desktop interaction and embedded RDP increment, 2026-10-10. Earlier reports are retained privately; this report supersedes their current-state claims.
Protocol remains v0: big-endian u32 framing, UTF-8 JSON, 1 MiB frame cap, owned private Unix socket and same-UID peer authentication. Correlation and stable IDs are unchanged. No public TCP control endpoint is introduced.

- pane.create accepts orientation horizontal or vertical and an optional target pane within the window. Default horizontal preserves compatibility. Maximum 16 panes per window.
- pane.layout accepts window with zoom boolean, or split ID and ratio 100..900. Unknown split IDs fail STALE_ID. Layout is daemon-owned.
- client.render adds layout tree, zoom and each pane rect (x/y/rows/cols). Grid adds mouse_drag and mouse_motion to the existing mouse/mouse_sgr flags.
- pane.history accepts pane, offset and lines (1..500), with optional epoch. Response has text, next_offset, total_lines, epoch, wrapped and alternate_screen. Text pages are capped at 128 KiB. Mismatched epoch fails STALE_HISTORY; clients must retry rather than join incoherent pages.
- client.input kind mouse accepts structured mouse_event button/x/y/release. Canonical mode selects SGR or X10. Disabled reporting sends no application bytes. Legacy coordinates outside the byte range are discarded.

Existing pane.capture (maximum 2,000 lines), input/text limits and internal buffer (256 KiB) remain compatible. Desktop clipboard text is separately capped at 1 MiB and sent in acknowledged UTF-8 chunks. PNG data never enters JSON text transport: bounded binary streaming and validated private staging remain separate from attachment success.

Physical GNOME keyboard/touchpad/PRIMARY acceptance is unqualified. Weston does not provide PRIMARY. Real authenticated RDP requires an explicitly authorized endpoint, username, Secret Service profile and trusted server certificate/CA; none is designated. RDP CLIPRDR, dynamic remote resizing, clipboard image attachment acknowledgement, IME, accessible terminal semantics and RPM distribution remain unfinished. TUI copy remains bounded capture rather than the desktop interactive copy UI. Foreground/EOF terminal mouse and alternate-screen state reset is implemented; restoration of termios after an abruptly killed child is not certified. Daemon or host failure still loses in-memory sessions.
