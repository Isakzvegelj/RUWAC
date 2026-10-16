# RUWAC saved host actions and desktop continuity

Machine bookmarks offer compact SSH, Codex, Claude and Kimi actions. RDP service credentials stay separate from linked SSH accounts. Missing SSH configuration opens an explicit editor; no connection or key authorization occurs merely by showing the machine list. Agent menus include saved hosts; terminal context menus put recent SSH connections first.

Kimi defaults to `kimi --session`. Per-machine opt-in installation uses the selected provider’s official per-user installer. Provider authentication remains in its own terminal flow. Agents may share a named remote RUWAC session in independent windows; reattachment keeps the existing process and does not install or inject a changed command. This requires an updated remote CLI and an engine supporting independent client focus.

The desktop remembers normal dimensions, maximized/fullscreen state and saved workspace tabs. Terminal processes remain in the independent daemon. RDP connections still reconnect when their frontend process restarts; seamless RDP process handoff remains unfinished.

Ctrl+Shift+T opens a terminal and Ctrl+Shift+R opens RDP machine selection. `ruwac -r` accepts a name or exact session ID. `kill-session NAME_OR_SESSION_ID` closes the selected session’s programs, refusing ambiguous names. Repeated `Terminal` names belong to independent GUI-created sessions.

CLI status uses light blue. Native pane separators render after terminal backgrounds. New engines reserve divider cells so CLI lines never cover application text. Preserved older daemons retain their existing adjacent-cell layout; updates do not restart their live sessions.

Acceptance distinguishes generated authenticated SSH/PTY and isolated native GTK/Wayland tests from external host and physical input acceptance. The shipped RSA advisory and complete Fedora distribution acceptance remain open. This release does not claim full product completion.
