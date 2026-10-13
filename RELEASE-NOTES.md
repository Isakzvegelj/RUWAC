# Verified build provenance for Fedora testing archives

The existing hosted release job now generates a signed build attestation for the
exact tested archive using the commit-pinned official GitHub action. Publication
verifies the artifact digest, repository, signer workflow, tag ref, source commit
and hosted-runner identity with GitHub CLI. The signed bundle is retained beside
the archive for offline verification. Quality/privacy gates, immutable assets and
controller-owned versioning remain in the existing workflow.

Provenance establishes build origin under the GitHub/Sigstore trust model. It does
not clear the unresolved RSA advisory, qualify physical or external Windows input,
or replace signed Fedora RPM acceptance. Existing updater hash/manifest checks
remain unchanged; downloaded attestations support an additional verification step.
