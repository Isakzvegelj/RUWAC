# RUWAC source release

RUWAC is a compiled Rust terminal daemon/CLI and an optional native GTK4/libadwaita desktop with mixed terminal, embedded RDP and SSH/SFTP workspaces. Existing terminals and connections remain independent of desktop presentation.

This increment makes the native machine sidebar easier to use alongside RDP tabs. Saved machines receive most of its height, with creation, search, favorites and paging visible. Machine maintenance and Remmina import/review are grouped in native panels collapsed by default; the expanded import area scrolls within a bounded height. Existing credentials, endpoints and connection handlers are preserved.

Eight actual GTK library groups cover allocated layout and native panel toggling, all 1,000 endpoints across 20 pages, reviewed merge/Cancel/Undo, active/stale protection and restart. Full authenticated local RDP text/PNG/ownership/display/profile regression, headless and SSH checks accompany the stamped build. Synthetic widget actions do not qualify physical desktop input.

Validation distinguishes actual authenticated local RDP, GTK/GDK integration and real PTYs/SSH from physical input or external-host qualification. Detailed results are maintained in the acceptance report and sanitized evidence inventory.

The GitHub workflow verifies source/tag/package parity and creates the source release page. It does not compile binaries. Local build.sh compiles both executables; private per-user installation is available. Original source is Apache-2.0, and dependency licenses are preserved.

Stable qualification remains open: authorized Windows/Fedora RDP credentials and trusted endpoint identity, physical GNOME input/PRIMARY/HiDPI, mixed GNU Screen/tmux nesting, remaining agent/file/automation features, and qualified Fedora package/binary distribution. Reachable RDP/SMB ports do not prove login or file-share interoperability. Earlier live daemons and graphical clients are preserved during installation; new features apply to newly started matching processes. Checkpoints recover ended history, not running programs; hidden primary history during alternate-screen use remains an open limitation.
