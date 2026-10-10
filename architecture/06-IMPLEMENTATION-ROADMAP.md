# Implementation roadmap

The bounded slice and its measured gates determine whether to expand. Established tmux remains the fallback. No full replacement promise follows from one successful local run.

| Milestone | Prerequisite | Work and exit evidence |
| --- | --- | --- |
| A — TUI reliability | startup/literal/lifecycle gates | interactive selection/search, both split axes, per-pane coordinate mapping, keymap reload, full Screen semantics, real Vim/less/htop/fzf visual tests, nested and soak runs |
| B — desktop bridge | A; authorized Fedora graphical workstation | distinct internal/CLIPBOARD/PRIMARY, native clipboard MIME, physical gestures, explicit permission policy, local synthetic PNG consumed by a supported application |
| C — remote automation | A; designated second host | existing SSH adapter with JSON stdin, validated destinations, bounded file transfer/hash receipts, disconnect lifecycle, durable retry contract if needed |
| D — optional GUI | B; renderer/protocol stabilized | separate GTK4 frontend, native focus/clipboard, packaging independent of daemon/TUI; no GUI dependency in server |
| E — release | A–C gates, license/name decisions | dependency provenance/notices, CI, branch protection, reproducible Fedora package and separately verified macOS TUI; owner-selected version and release policy |

A precedes B/C. B precedes native GUI work; A–C precede release. Graphics requires a separate image-state/replay architecture and must not be confused with file attachment. If correctness regresses, stop expansion. If API median exceeds 2× tmux, profile the actor, process overhead and grid serialization; consider a tmux-backed frontend or the bounded ttmux startup repair.
