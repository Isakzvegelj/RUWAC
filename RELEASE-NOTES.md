# RUWAC navigation, installation and functionality review

Alt+M opens Machines and focuses search; Alt+A opens saved and running agents even with collapsed header actions. Generic terminal tabs identify the local computer. Connection indicators now have cached full-colour, shape-distinct states, and the terminal status bar uses the stronger default tab-accent blue.

The desktop no longer builds a detailed RDP test snapshot for every tab-status refresh. The reviewed TUI shell-exit regression is corrected: a normal exit no longer deletes the last session and then fails with STALE_ID; ended history remains reattachable. Existing idle/hidden terminal polling and unchanged-content redraw suppression are retained. Font cells keep fractional Pango width, and Preferences adds Tight line spacing while preserving saved appearance choices.

A dedicated workspace app icon, terminal/SSH/RDP/SFTP launcher keywords and `man ruwac` are installed through per-user and Fedora RPM delivery. Codex shortcuts use a distinct RUWAC-owned vector glyph. Existing generated launchers migrate without overwriting unrelated files.

Existing-key SSH discovery adds separate SSH endpoints only after authenticated public-key login; unknown host trust and rejected keys do not become successful bookmarks. Verification probes cannot reuse connection-multiplexing masters. RDP service credentials remain separate; this is not automatic remote public-key authorization.

See `spec/RUWAC_FUNCTIONALITY_AUDIT.md` for the app purpose, 39 requested capability areas, implemented/partial/deferred scope and review findings. The recent SSH image transfer's pane binding and bounded I/O, heuristic PTY output draining, broader external RDP and physical-input acceptance remain open.

Qualification uses private local PTYs, generated SSH outcomes, real authenticated loopback SSH/RDP and bounded GTK/Wayland fixtures. Exact receipts remain private. GitHub source-quality/privacy and artifact availability must be checked independently. The existing RSA advisory remains unsuppressed; testing packages are not complete Fedora distribution or security qualification. Existing tags and historical release descriptions are unchanged.
