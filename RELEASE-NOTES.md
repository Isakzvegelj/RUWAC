# RUWAC named sessions and nested terminal usability

Create and enter a persistent terminal session with `ruwac -S NAME`; reconnect to an
existing exact name with `ruwac -r NAME`. Multiple simultaneous clients retain their
own selected windows/panes. Duplicate names require an explicit ID, noninteractive
shortcuts are refused, and recursive self-attachment is rejected before raw mode.

The TUI status bar lists windows and marks the selected one. The terminal title
shows the session and current window and restores the surrounding title on detach.
Double Ctrl+A forwards the prefix to a nested terminal in TUI and GTK. Ctrl+A then o
selects the previous window; the legacy Screen double-prefix action is configurable.
Real PTY nesting tests include two through five RUWAC layers, and native GTK tests
exercise actual tmux with generated sessions. GTK keyboard callbacks are synthetic.

Fresh GTK workspaces create a separate terminal instead of borrowing an unrelated
CLI session. Shared sessions retain one canonical PTY size, governed by the smallest
attached view. Status text exposes that size. A single terminal no longer draws a
pane edge border. ANSI encoding and bold ANSI colors preserve terminal semantics;
indexed and RGB colors remain intact.

Remmina import discovers standard native, legacy and Flatpak profile directories
without asking for a filename. Selection previews metadata; explicit import remains
required. Credentials and private key paths are excluded, source files remain
unchanged, and custom file selection is retained.

Verified opt-in system RPM updates remain available. Existing build/release scripts
and controller own versions and publication. These are upstream Fedora testing
RPMs; signed Fedora repository distribution, dependency-security qualification and
complete external/physical acceptance remain separate. Advisories are unsuppressed.
