# RUWAC SSH image delivery reliability

Keep the existing local and remote image-paste workflow. In a saved SSH terminal,
Ctrl+V still uploads the PNG and pastes its remote path into the original pane.
Typing in that pane remains possible during upload. Local Codex paste and
explicit draft-attachment components are preserved.

Pending path delivery is now bound to the original pane, endpoint and clipboard
ownership. Pane/tab/window changes, reconnects, changed profiles and newer pastes
revoke it; completion never chooses another pane. The path is committed through
the existing guarded terminal paste protocol. Ordinary paths retain their text;
paths needing shell quoting are escaped.

Nonblocking upload drains stdin/stdout/stderr together, enforces response bounds
and one end-to-end deadline, and cleans up its owned SSH process group. Normal
frontend shutdown cancels active groups. The receiver checks image byte count
and SHA-256 before publishing a private file without replacing another file.
Saved SSH passwords/passphrases remain in Secret Service; background uploads do
not approve host identities or open authentication dialogs.

Qualification distinguishes generated pipe fault tests, actual loopback OpenSSH,
real GDK PNG ownership with synthetic local input, and actual provider/external
acceptance. The owner confirms existing local and remote paste works; this batch
does not infer a new external model-consumption or physical-input pass. A PTY
status-colour test now waits for the complete asynchronously read status row.

Persistent SSH launch associations across frontend replacement, heuristic PTY
output draining, broad Windows/Fedora interoperability, physical input and full
Fedora distribution qualification remain open. The RSA advisory remains
unsuppressed. Existing release scripts and their controller own versioning and
publication; existing tags and historical release descriptions are preserved.
