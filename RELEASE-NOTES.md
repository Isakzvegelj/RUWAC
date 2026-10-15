# RUWAC split exit and TLS security fixes

Exiting a shell in a split now closes that completed pane and expands the surviving layout. CLI and GTK share the same policy and use the existing authenticated pane-close operation, including with older running daemons. Surviving shell processes and concurrent attachments remain running. A closed split has ordinary pane-close history semantics; the final ended session and restored history remain inspectable.

The desktop lockfile updates rustls to 0.23.46, the upstream patch for RUSTSEC-2026-0346 through RUSTSEC-2026-0349. The unrelated RSA advisory remains unsuppressed. This remains a testing distribution with separate external Windows/Fedora and physical-input acceptance gates.

Installing an update leaves running windows unchanged. Restarting the frontend still reconnects its RDP transports; persistent terminal processes survive. The release does not claim seamless RDP process handoff.
