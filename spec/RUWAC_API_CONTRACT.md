# RUWAC local API additions

Embedded RDP and clipboard increment, 2026-10-11. Earlier reports are retained privately; this report supersedes their current-state claims.
Protocol remains v0: big-endian u32 framing, UTF-8 JSON, 1 MiB frame cap, owned private Unix socket and same-UID peer authentication. Correlation and stable IDs are unchanged. No public TCP control endpoint is introduced.

- pane.create accepts orientation horizontal or vertical and an optional target pane within the window. Default horizontal preserves compatibility. Maximum 16 panes per window.
- pane.layout accepts window with zoom boolean, or split ID and ratio 100..900. Unknown split IDs fail STALE_ID. Layout is daemon-owned.
- client.render adds layout tree, zoom and each pane rect (x/y/rows/cols). Grid adds mouse_drag and mouse_motion to the existing mouse/mouse_sgr flags.
- pane.history accepts pane, offset and lines (1..500), with optional epoch. Response has text, next_offset, total_lines, epoch, wrapped and alternate_screen. Text pages are capped at 128 KiB. Mismatched epoch fails STALE_HISTORY; clients must retry rather than join incoherent pages.
- client.input kind mouse accepts structured mouse_event button/x/y/release. Canonical mode selects SGR or X10. Disabled reporting sends no application bytes. Legacy coordinates outside the byte range are discarded.

Existing pane.capture (maximum 2,000 lines), input/text limits and internal buffer (256 KiB) remain compatible. Desktop clipboard text is separately capped at 1 MiB and sent in acknowledged UTF-8 chunks. PNG data never enters JSON text transport: bounded binary streaming and validated private staging remain separate from attachment success.

Physical GNOME keyboard layouts, AltGr, touchpad gestures, IME and PRIMARY remain unqualified; the owned Weston compositor does not provide PRIMARY. No authorized external Windows or GNOME RDP endpoint was designated, so those host interoperability and physical-desktop gates remain blocked. Dynamic remote resolution/reactivation, panning a desktop larger than the native-pixel viewport, file/HTML clipboard transfer, agent attachment acknowledgement and qualified RPM distribution remain open. In-memory terminal sessions survive client exit, but not daemon or host failure. General abrupt-child termios recovery remains unqualified beyond the owned Bash/job-control regression. The machine library currently imports nonsecret metadata and opens endpoints; a full profile editor and Secret Service management UI are still needed. Product distribution licensing remains unresolved before binary packaging.

## Native RDP boundary

No daemon method, wire framing or stable ID changes are introduced in this increment. FocusBroker adds local-to-remote and remote-to-local policies; its metadata gate remains independent of GTK and network dependencies. The desktop worker uses bounded typed commands for input, format announcements, requests and replies. CLIPRDR bytes travel on the negotiated RDP channel; image data is never tunneled through JSON or replaced with a staged-file path. Desktop to terminal paste still uses the existing authenticated, acknowledged text path.
