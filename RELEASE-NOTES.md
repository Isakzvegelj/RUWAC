# RUWAC downloadable Fedora testing bundles and update delivery

The existing tagged release workflow now compiles Fedora 44 x86_64 applications,
retains dependency notices, scans the complete bundle and attaches the archive and
checksum to the existing controller-created release. Publication waits for quality
and privacy checks on that exact source commit. Retries never overwrite an existing
asset with different bytes. Version assignment, commits and tags remain in the
ordinary build/release workflow.

Updates adds an explicit Use Fedora testing bundles option. Source builds remain
the default. Bundle delivery verifies the expected repository/tag/name, GitHub's
SHA256 asset digest, bounded download/extraction, exact source commit and every
manifest file before the existing immutable per-user installer executes. Missing
assets or pending quality checks report a pending update; they do not claim success
or silently start a local compilation. Switching delivery off during download
preserves the current installation. Rollback continues disabling automatic installs.

The archives are testing candidates, with the unresolved desktop RSA advisory
visible in their manifests. They are not signed RPMs or security-qualified releases.
Physical input and external Windows display/clipboard remain separate acceptance
requirements. Existing terminal, profile, clipboard and RDP functionality is retained.
