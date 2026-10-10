# RUWAC remaining product work

Desktop interaction and embedded RDP increment, 2026-10-10. Earlier reports are retained privately; this report supersedes their current-state claims.
1. Validate physical GNOME keyboard layouts/AltGr, touchpad gestures, divider dragging and PRIMARY on an owner-approved test seat. Add full Unicode/multiline/wrapped grid-drag coverage, IME and accessible terminal semantics.
2. Exercise authorized Windows and Fedora RDP hosts: authentication, server versions, real framebuffer performance, pointer and send-key behavior. Add CLIPRDR text/image with broker ownership and echo-loop control; handle remote resize/deactivation where supported.
3. Certify abrupt-application termios restoration without changing live raw applications; extend mouse exit/focus/close cards and prefix/chooser physical navigation.
4. Improve copy search next/previous occurrence, snapshot consistency under continuous output and TUI interactive selection; keep bounded payloads and visible backpressure.
5. Qualify RPM packaging/installation and consumer acknowledgement for agent image attachments. Preserve source release independently of runtime promotion.

Physical GNOME keyboard/touchpad/PRIMARY acceptance is unqualified. Weston does not provide PRIMARY. Real authenticated RDP requires an explicitly authorized endpoint, username, Secret Service profile and trusted server certificate/CA; none is designated. RDP CLIPRDR, dynamic remote resizing, clipboard image attachment acknowledgement, IME, accessible terminal semantics and RPM distribution remain unfinished. TUI copy remains bounded capture rather than the desktop interactive copy UI. Foreground/EOF terminal mouse and alternate-screen state reset is implemented; restoration of termios after an abruptly killed child is not certified. Daemon or host failure still loses in-memory sessions.
