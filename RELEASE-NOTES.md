# RUWAC CLI login-shell compatibility

New CLI sessions and TUI windows/splits now load the selected login shell explicitly, including when attached to a preserved older daemon. This covers one-command named sessions, detached creation, named session/window/pane commands, Screen control aliases and interactive split/window shortcuts. Existing shells retain their processes and startup settings.

Remote named creation obtains the default shell from the authenticated SSH endpoint. Update the remote RUWAC CLI as well; endpoints without shell discovery receive an explicit update diagnostic before creation. Raw RPC and JSON requests retain their literal arguments, including custom commands. Reattachment does not require shell discovery.

Qualification includes actual Bash profiles, generated shells, real PTYs, isolated current/older daemons and a disposable authenticated loopback SSH server. Ten synthetic peer cases exercise remote discovery refusals. Separate external-host and physical-input acceptance gates remain.

The earlier rustls patch affects the development test-server graph; the shipped RDP client uses native TLS. The independent RSA advisory remains visible and unsuppressed. This release does not claim seamless RDP frontend handoff or fully qualified Fedora distribution.
