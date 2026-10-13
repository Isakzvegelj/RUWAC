# RUWAC single-operation native paste

Native GTK clipboard paste stages UTF-8 chunks on the authenticated attached
connection, then commits one complete bounded PTY write with one bracket pair.
The daemon accepts at most 1 MiB per paste and sixteen simultaneous uploads.
Nothing is sent before commit. Abort, disconnect, detach and a 30-second deadline
release staged data; malformed offsets, oversized input, foreign tokens and
embedded end markers are rejected. A timed-out commit is never replayed.

Actual daemon/PTy tests cover a 1 MiB Unicode paste, staging isolation, duplicate
commit refusal, cancellation, limits, disconnect cleanup and the elapsed deadline.
An actual GDK clipboard peer and compiled GTK frontend on owned Wayland verify
one wrapper pair with exact content. Existing plain clipboard tests remain.
Older daemons accept small single-operation paste; large paste reports that the
daemon needs updating rather than silently splitting a user operation.

This is testing artifact qualification. Physical input, remaining external RDP
interoperability, signed RPM repository, unattended RPM installation and the
unsuppressed desktop RSA advisory remain separate open gates.
