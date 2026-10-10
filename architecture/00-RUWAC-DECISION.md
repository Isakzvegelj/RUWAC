# ruwac decision: independent Rust spike

2026-10-10. Status: bounded experiment; no production or desktop acceptance.

The previous evaluation conditionally preferred tmux plus a frontend. That conclusion remains valid: coverage scores were not maturity rankings. The owner now chooses an independent Rust vertical slice to test consistent daemon/client/API ownership. This is a strategic preference, not proof that established cores are inadequate. If correctness or the 2× tmux API latency target fails, profile and reconsider tmux or a ttmux repair before expanding.

All ten historical reports remain byte-preserved under `evaluation/reports`. Their findings include exact literal input and stable targets in tmux and ready-state ttmux, ttmux startup input loss in 5/5 probes, and unresolved cmux licensing and GUI boundaries. The historical five-sample process RTT measurements do not establish input-to-render latency. Desktop clipboard, PRIMARY, physical touchpad, actual image attachment and second-host SSH remain separate unverified gates.

Component comparison:

| Candidate | Terminal state | License boundary | Build and integration | Decision |
| --- | --- | --- | --- | --- |
| alacritty_terminal 0.26.0 + vte | Mature grid, scrollback, modes, Unicode, alternate screen | Apache-2.0 / Apache-2.0 OR MIT | Rust; canonical grid accessible without GUI | Use; verify focused state tests before expansion |
| Ghostty pinned 7e880f6 | Rich terminal including graphics | MIT component; enclosing cmux terms differ | Zig 0.16 plus embedding/FFI; historical cmux build 8m26s | Defer; graphics integration requires separate replay design |
| ttmux pinned 8740639 | Ready Rust multiplexer | MIT | Fast binary; client capability probe loses early input | Design reference only; no copied core |
| cmux pinned 11ff257 | Ghostty-based terminal hosts, broad typed API | Root GPL-3.0/BUSL cloud versus Cargo MIT requires resolution | Large build, desktop/host abstraction | Design reference only |

Other names already exist: [smithersai/zmux](https://github.com/smithersai/zmux) describes a Zig PTY daemon with Unix JSON-RPC; [thinkter/zmux](https://github.com/thinkter/zmux) describes a Rust GPUI terminal workspace and carries a [GPLv3 license](https://github.com/thinkter/zmux/blob/main/LICENSE). These are references, not dependencies or copied implementations. Their descriptions do not establish our detach, byte-integrity or desktop acceptance gates. The product has been renamed to ruwac. Distribution-license and broader namespace decisions remain open before binary distribution.

The repository contains product source and technical documentation. Compilation, caches, raw receipts and test state belong in an external build workspace. The repository is `w3fans/ruwac`.

## Spike outcome

GO for the next bounded reliability milestone: seven focused unit tests and eleven real-daemon integration groups passed, including 100 startup races without byte loss. All twelve 30-sample API medians met the 2× tmux target on the measured idle VM. This does not overturn the earlier tmux recommendation for a mature foundation or establish desktop readiness. Full replacement, production deployment and production binary distribution remain NO-GO pending the open gates. See acceptance and benchmark reports for exact scope and limits.
