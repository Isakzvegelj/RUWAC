# RUWAC source release

RUWAC is a compiled Rust terminal daemon/CLI and an optional native GTK4/libadwaita desktop with mixed terminal, embedded RDP and SSH/SFTP workspaces. Existing terminals and connections remain independent of desktop presentation.

This increment adds reproducible mixed terminal compatibility qualification. Sixteen real private GNU Screen/tmux/RUWAC chains cover two-to-five layers, both common Control-A and native tmux Control-B configurations, exact Unicode input and literal prefix routing, deep window creation/selection, exact resize propagation and detach/reattach with the same leaf PID/start time. The existing terminal implementation is preserved.

GNU Screen 5.0.1 still interprets its own Control-A detach command inside a pasted payload despite forwarding bracketed-paste markers. A separate fixture reproduces this upstream boundary; arbitrary-control paste through Screen is not qualified. tmux/RUWAC chains contain both control prefixes correctly. Use the explicit leaf RUWAC pane API when bypassing intermediate key parsers is required; never assume terminal paste bypasses Screen commands.

Normal build tests run the mixed matrix when GNU Screen and tmux test tools are installed and explicitly report a blocked qualification otherwise. They are not runtime dependencies of the daemon or CLI. Native library, RDP and authenticated SSH regressions remain part of the candidate workflow; physical input and external RDP remain separate gates.

Validation distinguishes actual authenticated local RDP, GTK/GDK integration and real PTYs/SSH from physical input or external-host qualification. Detailed results are maintained in the acceptance report and sanitized evidence inventory.

The GitHub workflow verifies source/tag/package parity and creates the source release page. It does not compile binaries. Local build.sh compiles both executables; private per-user installation is available. Original source is Apache-2.0, and dependency licenses are preserved.

Stable qualification remains open: authorized Windows/Fedora RDP credentials and trusted endpoint identity, physical GNOME input/PRIMARY/HiDPI, mixed GNU Screen/tmux nesting, remaining agent/file/automation features, and qualified Fedora package/binary distribution. Reachable RDP/SMB ports do not prove login or file-share interoperability. Earlier live daemons and graphical clients are preserved during installation; new features apply to newly started matching processes. Checkpoints recover ended history, not running programs; hidden primary history during alternate-screen use remains an open limitation.
