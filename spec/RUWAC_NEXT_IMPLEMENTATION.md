# RUWAC remaining product defects

## Current native shortcuts, appearance and safe diagnostics — 2026-10-11

The native desktop now captures Alt+1 through Alt+9 and Alt+0 at window level, selecting workspace positions 1–10 across terminal, RDP and Files views. Missing positions are a no-op. Ctrl+Alt/AltGr-style combinations, Shift and Super chords are not captured; Caps Lock is allowed. Tab selection retains existing focus/clipboard ownership handling and persistent connections.

The appearance follows the owner’s installed Ptyxis reference: native dark libadwaita chrome, a compact header, and workspace/PNG actions on a separate toolbar. `--theme dark|light|system` selects the desktop appearance (dark is the default). This is incremental styling of the existing compiled GTK client; the terminal engine and application-supplied terminal colors remain unchanged.

`ruwac doctor` provides bounded read-only socket ownership/permissions and daemon-health checks. `--desktop` checks executable/display/session-bus availability, without claiming a working graphical connection from environment variables alone. `--terminal` inspects only stdin’s TTY. Explicit `--terminal --repair` restores canonical input, echo, signals and newline processing only when this process owns the invoking TTY foreground group. Queued input/custom control characters are retained; no terminal escape reset is emitted. Existing paths are never repaired or deleted. Remote read-only diagnostics use the authenticated SSH transport; remote repair is refused.

The full product is not complete. Remaining gates include external Windows/Fedora RDP and physical GNOME input, stable view IDs and permission-scoped desktop automation, durable daemon-crash history, real agent image consumers, recursive/resumable files and encrypted-key UI, broader accessibility, matched/idle performance and qualified RPM/binary publication. Desktop styling still needs refinement beyond the current dark/compact native presentation. The authorized Windows endpoint is waiting for a credential entered through the profile editor; no password is inferred from “same as here.”


## Current authenticated host CLI and discovery — 2026-10-11

`ruwac --host SSH_HOST` now sends named commands, raw RPC and JSON stdin requests through authenticated OpenSSH. `--remote-binary` selects the endpoint executable; an explicit `--socket` selects the endpoint socket. Without it, the remote CLI chooses its own private default: the local RUWAC_SOCKET is not forwarded. Existing SSH config and keys are used with BatchMode and strict host-key checking; agent and configured port forwarding are disabled. Request data travels as JSON on stdin, never as a remote shell argument. Executable/socket arguments are individually shell-quoted. Destination/option validation rejects malformed hosts, duplicate globals and remote desktop actions.

`--host HOST start` starts/reuses the remote daemon only when explicitly requested. `--host HOST attach --session ID` opens a real SSH PTY, retaining daemon sessions after detach. Control does not implicitly start a daemon. Remote reply version/correlation and 1 MiB bounds are checked. Control has a 70-second response deadline and a two-second post-EOF exit bound; failure terminates/reaps the SSH process. Interactive attachment is intentionally governed by the user’s session lifetime.

`describe` is an additive read-only daemon method exposing schema, capabilities, version and transport boundaries. The CLI probes the schema first and can aggregate existing read-only methods for an older live daemon, returning native_describe=false without inventing capabilities or restarting its processes. Discovery does not change daemon revision. The daemon/CLI remain independent of GTK, IronRDP and libssh2.

Remaining full-product gates include stable GUI view IDs and permission-scoped GUI/RDP automation, durable daemon-crash metadata/history, actual agent image consumers, recursive/resumable files and encrypted-key UI, external Windows/Fedora RDP and physical GNOME acceptance, complete nested Screen behavior, idle-qualified/matched performance and qualified RPM/binary publication. Host control requires a compatible endpoint CLI, pre-established SSH authentication and verified host key; it does not provision credentials or forward a desktop. Original source remains Apache-2.0. No shared release infrastructure or manually allocated versions were introduced.


## Current individual view management — 2026-10-11

**Views…** opens a native dialog for the selected mixed workspace. Each leaf offers **Move to own tab** and **Close view**. Removing a leaf collapses its divider and keeps the sibling widget/tree alive. Moving preserves the underlying connection; closing a terminal detaches its presentation while daemon programs keep running, closing RDP disconnects only that connection, and closing Files cancels its work and releases its own lease. Closing the last leaf removes the empty tab. Dialog errors remain visible if a view disappeared. The dialog takes local focus before opening; remote clipboard ownership cannot follow it.

Connection registries remove only the selected leaf. Profile deletion protection survives a move and remains until the last associated view closes. Workspace schema 2 records the resulting topology without a schema change; no daemon protocol, engine or dependency replacement was introduced. Existing terminal splits inside a session remain independent of mixed GUI leaves.

Full product completion remains unproven. Stable persisted GUI view IDs and exact per-leaf focus restoration, permission-scoped automation/host CLI, durable daemon-crash history, supported local/remote agent image consumers, recursive/resumable files and encrypted-key UI, external Windows/Fedora RDP interoperability, physical GNOME input/accessibility, idle-qualified/matched performance and qualified RPM/binary publication remain open. No external RDP login is claimed. Original source remains Apache-2.0; dependencies retain their licenses.


## Current mixed workspace increment — 2026-10-11

Remaining product limits: individual mixed-pane close/move UI, stable GUI view IDs and exact per-leaf focus restoration; durable terminal history after daemon loss; permission-scoped GUI automation and host CLI; supported local/remote agent image consumption; encrypted SSH-key passphrase UI, recursive/resumable/rsync files; physical GNOME PRIMARY/AltGr/IME/touchpad/accessibility; external Windows/Fedora RDP login/clipboard and matched Remmina/FreeRDP performance; qualified RPM/binary publication and broader governance/CI. Authorized external endpoints are identified, but credentials must be entered through the owner’s profile editor and certificate identity must validate. No owner credential store was used in tests. Apache-2.0 is now approved for original source; dependency notices and packaging qualification remain separate.

Continue independent implementation while external credentials/physical seats are unavailable. Priorities include mixed-pane management, durable ended-session metadata/history and actual supported agent image consumption. The full product objective remains active.

## Earlier milestone evidence

External Windows/Fedora-GNOME RDP and physical GNOME PRIMARY/AltGr/IME/touchpad acceptance remain BLOCKED: RDP credentials, matching certificate identity and physical-seat qualification remain unavailable; designated hosts now include an SSH-accessible Fedora target and a Windows target. Loopback authentication, GDK and server-driven reactivation are real integration results, not external-host acceptance. The raw bitmap fixture does not qualify the GNOME/RemoteFX degradation described in [IronRDP #447](https://github.com/Devolutions/IronRDP/issues/447).

Known product limits: gateway/network fields are metadata, not a routing engine; Remmina conflicts support skip or new identity, without merge-by-ID; group default editing covers display/clipboard while other endpoint overrides remain explicit. Profiles used by connection tabs are deletion-protected until all their active tabs close; reference-counted leases release protection without an application restart. SSH terminals use OpenSSH's interactive verification/authentication; Files uses the configured key/agent or Secret Service password, and refuses unknown/changed keys. Encrypted private-key passphrase UI, rsync, recursive transfers and resume are not implemented. Portable SFTP v3 replacement preserves the old file through a backup/restore sequence but is not atomic. A lost connection can leave a named remote partial or backup; connected cancellation cleans its partial. Concurrent changes to a remote namespace are not an atomic SFTP snapshot. Native file chooser upload/download gestures and locked-keyring dialogs still need interactive desktop acceptance; transport integrity uses the production worker through an isolated driver, while panel listing/refresh uses the actual GTK window. File/HTML CLIPRDR, consumer acknowledgement for agent attachments, general termios recovery beyond Bash and daemon/host-crash persistence remain open. RPM/binary publication and product distribution licensing remain unqualified.

Priorities: designated Windows/Fedora and physical-seat qualification; negotiated RemoteFX cadence/CPU; import merge-by-ID and route execution; endpoint-use lifecycle precision; encrypted SSH key passphrases; reconnect-time partial cleanup/resume; package qualification and owner distribution-license decision. Existing terminal/clipboard engines remain the foundation.


## Usable desktop and authorized SSH acceptance — 2026-10-11

External Windows/Fedora GNOME RDP login and clipboard interoperability are unverified. The authorized Fedora host is reachable using an existing SSH key and advertises RDP port 3389; its configured certificate has CN=GNOME and no subjectAltName, so hostname validation needs a supported trust solution. No password has been received. Physical PRIMARY/AltGr/IME/touchpad qualification, mixed terminal/RDP/Files splits, complete workspace/crash restoration, agent image consumption, encrypted SSH key passphrases, rsync/resume/recursive transfers and RPM/binary distribution remain open. The owner source-license decision is pending.

Priorities: external RDP trust/credentials and interoperability; mixed workspace and complete restoration; agent/image consumption; resumable file operations; install/RPM qualification and governance. The full product objective remains active.
