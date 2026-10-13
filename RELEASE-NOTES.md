# RUWAC Fedora lifecycle and remote launcher compatibility

Fresh Fedora 44 testing now covers actual DNF dependency resolution, installation,
matching core/desktop upgrade, rollback to a genuine prior build and removal.
The daemon and raw-receiver PTY survive upgrade/rollback; generated private
configuration is preserved. A reusable opt-in driver runs in a bounded disposable
RAM filesystem and removes its own containers/mount without touching the host
package database, homes, desktop or RDP sessions.

Remote Codex launch preserves an existing cx alias/function. Without one, a
read-only help probe chooses the supported agents browser or the ordinary TUI
without a positional prompt. Actual process-boundary regressions cover both CLI
variants, command precedence and unavailable Codex; quoted bootstrap targets
remain supported. Tests do not open interactive external agent threads.

System RPM updates use DNF. Per-user checked bundles retain opt-in unattended
updates; the two installation modes remain separate. Physical input, Windows
interoperability, complete external interactive launcher acceptance, a signed
RPM repository and desktop RSA remediation remain open. These testing artifacts
do not establish whole-product security/stability qualification.
