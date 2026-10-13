# RUWAC independent terminal client focus

Attached terminal clients now retain independent selected windows, panes, previous
windows and zoom state. One client’s navigation no longer redirects another
client’s input. Split topology, canonical terminal state and PTYs remain shared.

Shared PTYs use the documented minimum attached and visible-pane sizing policy.
Closing a target repairs affected views without replacing surviving processes.
New attachments use the persistent canonical default. Selection idempotency uses
a fresh attachment namespace; shared creation and text mutations retain global
deduplication. A cached creation does not replay UI focus onto a second client.

The generated integration fixture tests two real TUI processes, same-packet
prefix/input routing, independent selection and zoom, minimum sizing, stale
target repair, cross-session refusal and both idempotency policies. Existing
terminal, GTK, clipboard, profile, file and authenticated local RDP regressions
remain part of the normal validation workflow.

Installing new binaries does not restart existing GUI windows or daemons. Older
running daemons retain their existing behavior until deliberately replaced.
Physical desktop input, Windows dynamic display/external clipboard and qualified
Fedora RPM distribution remain separate acceptance gates. Testing archives are
not qualified RPMs. RUSTSEC-2023-0071 remains visible and unsuppressed.
