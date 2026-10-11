# RUWAC source release

RUWAC is a compiled Rust terminal daemon/CLI and an optional native GTK4/libadwaita desktop with mixed terminal, embedded RDP and SSH/SFTP workspaces. Existing terminals and connections remain independent of desktop presentation.

This increment completes bounded headless interactive copy/search, Unicode marks, rectangular selection, physical soft-wrap joining, word/page movement, window chooser, command prompt, help, zoom and configurable Control prefixes. Modal paste/mouse input cannot reach running applications. Cancellation and oversize selections preserve the previous internal buffer.

Canonical copy uses immutable, connection-owned history snapshots, retained for at most 30 idle seconds and reclaimed on disconnect. Each snapshot is bounded to 50,000 physical rows or 8 MiB, with a global eight-pin cap. GTK copy uses this protocol when available and retains compatible fallback for older daemons. Nine real-PTY groups include actual two-through-five-layer RUWAC nesting; three API groups validate ownership, resize/output stability, limits and actual expiry. Mixed GNU Screen/tmux nesting remains a separate gate.

The Remmina-inspired native machine sidebar, visible RDP toolbar and Alt+1–9/0 workspace navigation remain available. Existing daemon sessions are preserved.

Validation distinguishes actual authenticated local RDP, GTK/GDK integration and real PTYs/SSH from physical input or external-host qualification. Detailed results are maintained in the acceptance report and sanitized evidence inventory.

The GitHub workflow verifies source/tag/package parity and creates the source release page. It does not compile binaries. Local build.sh compiles both executables; private per-user installation is available. Original source is Apache-2.0, and dependency licenses are preserved.

Stable qualification remains open: authorized Windows/Fedora RDP credentials and trusted endpoint identity, physical GNOME input/PRIMARY/HiDPI, complete headless copy/chooser/nesting, remaining agent/file/automation features, and qualified Fedora package/binary distribution. Reachable RDP/SMB ports do not prove login or file-share interoperability. Earlier live daemons and graphical clients are preserved during installation; new features apply to newly started matching processes. Checkpoints recover ended history, not running programs; hidden primary history during alternate-screen use remains an open limitation.
