# RUWAC split-divider qualification correction

Correct the independent-client focus fixture to expect the column reserved for a vertical split divider. The left pane is 49 columns while the unsplit pane remains 100 columns. Input isolation, per-client zoom and shared PTY sizing remain independently tested. The local project build and frozen release qualification now run this focus fixture.

The previous release added Kimi and optional agent installation, compact saved-host shortcuts, shared agent windows, saved desktop geometry/workspaces, exact-ID session controls and light-blue CLI status. This correction preserves those features. No published tags or history are rewritten.

Qualification uses owned local PTYs, GTK/Wayland and authenticated loopback RDP/SSH fixtures. Physical desktop and external-host acceptance remain separate. The RSA advisory remains unsuppressed, and this release does not claim complete Fedora distribution or product qualification.
