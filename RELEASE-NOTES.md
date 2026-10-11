# RUWAC source release

RUWAC is a compiled Rust terminal daemon/CLI and an optional native GTK4/libadwaita desktop with mixed terminal, embedded RDP and SSH/SFTP workspaces. Existing terminals and connections remain independent of desktop presentation.

This increment adds authenticated native RDP list/query/open/reconnect/disconnect operations. Lifecycle control requires explicit independent owner consent, disabled by default. Activation also requires focus consent. Background connection changes preserve neighboring transports, selected tabs and clipboard ownership. Saved-profile credentials resolve privately through Secret Service.

The headless CLI routes the same typed operations over verified SSH to an existing trusted GUI socket. Missing endpoints never start a GUI or daemon. Exact parameters, stable targets, stale-target errors, bounded replies and runtime revocation keep automation scoped.

Seven authenticated local RDP groups and three actual SSH GUI forwarding groups accompany existing library, headless and GTK regressions. The SSH fixture forwards to a locally rendered GTK window; remote-seat rendering and external Windows/Fedora login are separate acceptance gates.

Validation distinguishes actual authenticated local RDP, GTK/GDK integration and real PTYs/SSH from physical input or external-host qualification. Detailed results are maintained in the acceptance report and sanitized evidence inventory.

The GitHub workflow verifies source/tag/package parity and creates the source release page. It does not compile binaries. Local build.sh compiles both executables; private per-user installation is available. Original source is Apache-2.0, and dependency licenses are preserved.

Stable qualification remains open: authorized Windows/Fedora RDP credentials and trusted endpoint identity, physical GNOME input/PRIMARY/HiDPI, mixed GNU Screen/tmux nesting, remaining agent/file/automation features, and qualified Fedora package/binary distribution. Reachable RDP/SMB ports do not prove login or file-share interoperability. Earlier live daemons and graphical clients are preserved during installation; new features apply to newly started matching processes. Checkpoints recover ended history, not running programs; hidden primary history during alternate-screen use remains an open limitation.
