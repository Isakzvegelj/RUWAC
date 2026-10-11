# RUWAC source release

RUWAC is a compiled Rust terminal daemon/CLI and an optional native GTK4/libadwaita desktop with mixed terminal, embedded RDP and SSH/SFTP workspaces. Existing terminals and connections remain independent of desktop presentation.

This increment adds a Remmina-inspired machine sidebar with protocol icons and two-line profile rows, and a visible RDP toolbar for reconnect, disconnect, fit, dynamic resolution, zoom and fullscreen. Standard native theme icons are used; Remmina code and artwork are not copied. Advanced remote keys, clipboard policy and endpoint controls remain in the existing popover. Alt+1–9/0 still selects workspace tabs.

Screen's double prefix switches to the previous window; prefix followed by a plain `a` sends the literal prefix. The remembered window survives private checkpoints. Checkpoint schema 2 reads schema 1, preserves ended-history semantics and rejects invalid previous-window references. Canonical text expands tab cells once rather than emitting a tab followed by already-expanded blanks; raw cells retain the tab marker. Paged history exposes physical rows and wrap flags alongside the existing text response.

Validation distinguishes actual authenticated local RDP, GTK/GDK integration and real PTYs/SSH from physical input or external-host qualification. Detailed results are maintained in the acceptance report and sanitized evidence inventory.

The GitHub workflow verifies source/tag/package parity and creates the source release page. It does not compile binaries. Local build.sh compiles both executables; private per-user installation is available. Original source is Apache-2.0, and dependency licenses are preserved.

Stable qualification remains open: authorized Windows/Fedora RDP credentials and trusted endpoint identity, physical GNOME input/PRIMARY/HiDPI, complete headless copy/chooser/nesting, remaining agent/file/automation features, and qualified Fedora package/binary distribution. Reachable RDP/SMB ports do not prove login or file-share interoperability. Earlier live daemons and graphical clients are preserved during installation; new features apply to newly started matching processes. Checkpoints recover ended history, not running programs; hidden primary history during alternate-screen use remains an open limitation.
