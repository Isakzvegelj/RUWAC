# RUWAC compiled-update provenance enforcement

Per-user Fedora testing-bundle updates now verify the signed build attestation
before archive extraction or execution of the installer. Verification binds the
actual bytes to the RUWAC repository, release workflow, exact source tag/commit
and hosted-runner identity. Missing, ambiguous, invalid or timed-out attestations
hold installation. GitHub CLI uses isolated configuration without owner tokens.
Install gh through DNF before enabling compiled bundle updates.

Generated verifier-process faults check identity arguments, credential isolation,
invalid metadata and rejection before unpack/install. A genuine published archive
and attestation also exercise the production verification and immutable installer.
Existing opt-out, locks, rollback and session-preservation behavior remain intact.
The CI quality job runs the new regressions explicitly.

Fedora 44 core and desktop testing RPMs continue through the normal tagged build
and publication workflow. System RPM updates still use DNF; unattended RPM updates
and a signed Fedora repository remain unfinished. The RSA advisory is unsuppressed.
External Windows display/clipboard and physical desktop acceptance remain separate
from owned fixture passes. No remote system or live owner connection is changed.
