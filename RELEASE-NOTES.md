# RUWAC explicit SSH jump routing

Configure an existing OpenSSH gateway alias in the native endpoint or network-group
editor. Native SFTP, resumable files, rsync, SSH terminal and remote-agent launch
use the explicit route. Existing descriptive gateway metadata remains metadata.

Gateway key/agent authentication and strict trusted-host checks are independent
of target identity, known-hosts and saved credentials. Native file transports use
an anonymous socket pair and an owned OpenSSH child; no local listening port or
shared SSH master is created. Direct-transfer recovery identities are preserved;
a changed jump alias cannot silently resume an existing job.

Generated two-server fixtures exercise actual SSH, file integrity, cancellation,
resume, recursive rsync, terminal commands and both authentication/trust boundaries.
These do not establish external bastion, physical input or Windows display acceptance.
Existing RDP, terminal and clipboard regressions remain release gates. The RSA
advisory and signed Fedora RPM requirements remain open; publication is a source
release, distinct from qualified binary distribution.
