# RUWAC Fedora testing RPM publication repair

The existing release workflow now supplies an explicit checksum manifest containing
exactly the archive and both matching core/desktop RPMs to the attestation action.
Missing, extra, symlinked or mixed-version artifacts are refused before signing.
Every downloaded artifact remains subject to repository, workflow, source tag and
commit verification before publication.

The previous source tag is preserved. Its hosted build compiled and tested the
applications and assembled both RPMs, but signed only the archive; verification
correctly held all asset uploads. This repair changes the project workflow and
does not alter the shared release controller or invent a version.

The separate Fedora 44 testing RPMs preserve headless independence, dependency
notices, existing sessions and private configuration. System RPM updates use DNF;
per-user automatic installation/rollback is refused for an RPM installation.
These unsigned testing RPMs carry GitHub provenance and remain distinct from
an official signed Fedora repository or complete security qualification.
The desktop RSA advisory remains unsuppressed.
