# ruwac architecture and decisions

Current desktop interaction and embedded IronRDP state is recorded in [the acceptance report](../spec/RUWAC_ACCEPTANCE_STATUS.md) and [current architecture decisions](../spec/RUWAC_ARCHITECTURE_ADR.md). Earlier measurements and design limits below are historical evidence.


1. **PTY ownership:** portable-pty 0.9.0 opens real PTYs and spawns direct argv. The daemon owns master, input queue and child. Client detach never closes them. Process cleanup is limited to verified owned child groups; restart/reboot does not recover processes.
2. **Terminal state:** alacritty_terminal 0.26.0 owns each pane's grid and capped 2,000-line history. Its vte parser consumes output. Clients receive canonical cell snapshots, not arbitrary escape replay. Graphics are unsupported. OSC 52 clipboard events are ignored; application capability replies return to that application's PTY.
3. **Concurrency:** one bounded Tokio actor serializes session/window/pane state. Blocking PTY readers and writers run separately with bounded queues. Slow connections have frame and time limits. Spawn may briefly block the actor; benchmark before expansion.
4. **Protocol:** version-zero JSON in four-byte big-endian length frames, at most 1 MiB. Typed opaque IDs, structured errors, explicit targets and bounded captures. Acknowledged input is queued, not proof of application execution. Idempotency is bounded and daemon-lifetime only.
5. **Clipboard:** internal text buffer is daemon state. It does not read or alter CLIPBOARD or PRIMARY. Native desktop integration needs a separate frontend; outer-emulator shortcuts cannot be guaranteed by a TUI.
6. **Views:** session focus is shared. The smallest attached client dimensions define the session size; split panes divide usable rows. CLI-only requests do not resize sessions. Detachment is connection-local and PTYs survive the final detach.
7. **Input:** no startup capability probe. Incremental classification distinguishes keyboard bytes, recognized capability replies, SGR mouse and bracketed paste. Partial sequences survive read boundaries. Mouse routing checks the targeted pane's current mode inside the actor. Paste content bypasses prefix handling.
8. **Remote security:** Unix socket only, private directory/socket, same UID, exclusive server lock. Future remote control uses authenticated SSH with JSON on stdin, never shell interpolation of sent text. No custom TCP service.
9. **Image boundary:** local PNG validation, private staging and SHA-256 receipt only. Rendering, clipboard acquisition, remote transfer and an application actually attaching an image are distinct unimplemented operations.
10. **Naming and release:** ruwac is the product and CLI name. Eight internal unpublished crates share the tagged prototype version. The first prerelease publishes source; distribution license remains an owner decision before binary distribution.

Crates: protocol (wire contract), pty (child/PTY adapter), term (grid), core (state/layout), daemon (socket service), tui (Screen prefix/render/input), clipboard (PNG staging), cli (entry point). See the acceptance matrix for observed coverage rather than treating decisions as results.
