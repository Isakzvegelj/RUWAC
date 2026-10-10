# RUWAC acceptance — embedded RDP, CLIPRDR and machine sidebar

2026-10-11 increment from v0.1.2. Protocol-fixture, native desktop and external-host results are distinguished below. Prior reports and failed receipts are retained privately.

| Gate | State | Actual evidence |
|---|---|---|
| Headless regressions | PASS | 18 Rust units; 11 integration groups including 100 startup races |
| Layout/history/mouse API | PASS | Three real-daemon groups; nested layout/ratio/zoom, stale history rejection, exact SGR/X10/motion/disabled routing |
| Native terminal desktop | PASS fixture | 21 cards, including real Vim/less/htop, Unicode save, keyboard marking/search/internal paste, persistent splits and crash staging cleanup |
| PRIMARY | BLOCKED | Owned Weston lacks the protocol; physical GNOME PRIMARY remains unqualified |
| Embedded authenticated RDP | PASS loopback | Three simultaneous native tabs, actual TLS/CredSSP/NLA and decoded 320×180 server frames |
| Negative authentication/TLS | PASS integration | Unknown CA rejected; generated wrong password rejected before a frame; explicitly trusted local CA accepted |
| CLIPRDR text | PASS native loopback | 10 payloads per tab in both directions: 60 transfers; Slovenian/Polish/emoji/multiline, 10 KiB and 1 MiB |
| CLIPRDR images | PASS native loopback | Three cases per tab in both directions: 18 transfers; transparent 64-square/1440p PNG and DIBV5, decoded pixel hashes |
| Conversion/caps | PASS unit | PNG/DIBV5 alpha roundtrips; DIB RGB conversion, odd-width row stride, invalid UTF-16 and early fragment allocation caps |
| Active-tab ownership | PASS native loopback | 200 switches across three RDP tabs and terminal; 450 inactive offers never replaced the independent GDK peer sentinel |
| Clipboard policies | PASS native loopback | All five policies, directional rejection and explicit manual receive |
| Reconnect | PASS native loopback | Server-driven disconnect, bounded automatic retry in the same tab, decoded frame and clipboard transfer afterward |
| Machine sidebar/import | PASS native fixture | 70 full-group profiles with repeated leaf names; actual GTK preview/import/search/favorite, private persistence, source unchanged and secrets excluded |
| Preview special files | PASS unit | FIFO rejected without blocking; symlinks refused; regular file and storage caps |
| Raw child recovery | PASS bounded fixture | Owned Bash child SIGKILL, mouse reset/no leakage, ECHO/ICANON and exact Unicode input restored |
| Headless dependency boundary | PASS | Core Cargo graph and ELF links exclude GTK/RDP/libsecret |
| External Windows/GNOME RDP | BLOCKED | No designated endpoint or authorized account; loopback is not external-host qualification |
| Physical keyboard/touchpad/AltGr/IME | UNQUALIFIED | Synthetic callbacks and XTEST in an isolated compositor do not certify physical input |
| Dynamic remote resize / file clipboard | OPEN | Visible reconnect fallback retained; file/HTML formats not advertised |
| RPM/runtime promotion | BLOCKED | Source prerelease only; no qualified package handler or binary distribution decision |

1. Qualify an explicitly designated disposable Windows and Fedora/GNOME host. Exercise NLA, certificate handling, remote frames, send-key actions, clipboard consumers and disconnect/reconnect on those hosts. Loopback protocol results do not establish host interoperability.
2. Qualify physical GNOME keyboard layouts/AltGr, PRIMARY, touchpad gestures, divider drag, IME and accessibility on a test seat without capturing private clipboard contents.
3. Extend connection reactivation, negotiated display-control resizing and panning in native-pixel mode. Add deeper cancellation/slow-peer/abrupt-server failure cards. File/HTML CLIPRDR remains outside the supported text/image formats.
4. Add machine/profile creation and editing, endpoint management and Secret Service reference management. Preserve the explicit read-only import preview and full group identity. Qualify SSH against a disposable authorized SSH endpoint.
5. Extend general termios recovery beyond the passing Bash/job-control fixture, copy search navigation, coherent selection under continuous output and TUI interactive selection.
6. Qualify the existing packaging/installation handler and distribution notices before RPM/binary publication. Implement consumer acknowledgement for agent image attachments separately from PNG staging or clipboard transfer.
