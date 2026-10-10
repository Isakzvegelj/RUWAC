# RUWAC clipboard, input and RDP matrix

Desktop interaction and embedded RDP increment, 2026-10-10. Earlier reports are retained privately; this report supersedes their current-state claims.
| Gate | State | Scope |
|---|---|---|
| Screen navigation/keyboard mark/search/internal paste | PASS fixture | Compiled native GTK callbacks; physical input unqualified |
| Vim, less, htop | PASS fixture | Real programs, Unicode Vim save and actual GTK input handlers |
| Split ratio, nested orientations, zoom and reconnect | PASS | Real daemon API; native split/zoom callbacks; physical divider drag unqualified |
| GDK CLIPBOARD text and independent internal buffer | PASS | Separate GDK peer on isolated Weston; synthetic content |
| PRIMARY | BLOCKED | Compositor lacks protocol; no owner clipboard access |
| SGR modifiers / legacy X10 / motion / disabled routing | PASS | Canonical daemon modes and exact receiver bytes |
| PNG size/hash/caps/cancel/normal and crash cleanup | PASS | Private namespace locking and generated PNGs; staging only |
| Embedded RDP view and pointer transforms | PASS fixture | Same GTK window, synthetic framebuffer and pure geometry |
| RDP TLS validation | PASS | Real loopback RDP negotiation/TLS, trust rejection/explicit-CA continuation |
| Authenticated RDP host / performance | BLOCKED | Missing designated endpoint/secret/certificate fixture |
| CLIPRDR text/image ownership | UNIMPLEMENTED | Existing per-view broker boundary prepared; no remote transport yet |

Physical GNOME keyboard/touchpad/PRIMARY acceptance is unqualified. Weston does not provide PRIMARY. Real authenticated RDP requires an explicitly authorized endpoint, username, Secret Service profile and trusted server certificate/CA; none is designated. RDP CLIPRDR, dynamic remote resizing, clipboard image attachment acknowledgement, IME, accessible terminal semantics and RPM distribution remain unfinished. TUI copy remains bounded capture rather than the desktop interactive copy UI. Foreground/EOF terminal mouse and alternate-screen state reset is implemented; restoration of termios after an abruptly killed child is not certified. Daemon or host failure still loses in-memory sessions.
