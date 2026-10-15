# RUWAC compact desktop and machine workspaces

This batch introduces a compact single-row header, contextual Controls with optional
docking, six live color schemes and configurable terminal font/spacing. Machine
profiles are scrollable without pagination and groups remember their collapse state.

SSH profiles can expose Codex/Claude shortcuts with configurable resume commands
and RUWAC, Screen, tmux or direct sessions. Explicit remote CLI setup uses the existing
headless installer. New RDP profiles default to Dynamic resolution. Ctrl+Alt+Home
moves focus to local controls; Ctrl+Alt+Enter toggles fullscreen, leaving AltGr intact.

Read the desktop guide and current acceptance status for exact validation scope.
Generated fixtures and native GTK callbacks are separate from physical input and
external-host acceptance. Existing dependency advisories remain unsuppressed.

Known limitation: Weston 15.0.1 popup-after-resize compatibility remains unqualified.

The CLI status bar fills the bottom row and remains there across large-terminal
resizes, showing session/window identity before shortcut hints.
