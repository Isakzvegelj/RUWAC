# RUWAC Fedora testing-bundle preparation

Add a product-owned candidate bundle assembler and Git-free per-user installation.
Every bundled file is hashed before artifact execution. Symlinks, unexpected files,
wrong architectures, stale license provenance and modified registry license texts
are rejected. Immutable previous releases and running sessions are preserved.

Dependency notice generation retains original texts for the Linux runtime/build
closure and supplements omitted texts from their pinned upstream VCS commits.
Candidate archive headers are normalized and private-content/standard-secret checks
are mandatory. The assembler does not allocate versions or publish artifacts.

This remains a source release. Candidate bundles are not signed RPMs, publisher
attestations or stable security qualification. RUSTSEC-2023-0071 remains unresolved.
Windows dynamic display, physical input and qualified Fedora packaging remain open.
See spec/RUWAC_BUNDLES.md and spec/RUWAC_UPDATES.md for installation and limits.
