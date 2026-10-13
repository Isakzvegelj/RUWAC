# RUWAC Fedora testing RPM delivery

The existing tagged release job now assembles separate Fedora 44 x86_64 upstream
testing RPMs for the daemon/CLI and native desktop. The desktop requires the exact
core package and declares its runtime dependencies; the headless RPM has no GTK or
RDP dependency. Original and dependency licenses are preserved with each package.

Use build.sh --release --desktop --package rpm for local testing packages. The
release controller continues to assign the application version. RPM metadata uses
a generic build host and is checked for private disclosures; exact GitHub source
provenance is verified before attaching RPM assets through the existing workflow.

RPM transactions do not restart daemons, close windows or alter private profiles.
The RPM updater displays the release downloads and refuses per-user installation,
rollback and automatic-install settings that would shadow the system package.
System RPM updates currently require DNF; a signed automatic-update repository is
not yet implemented. Existing per-user update delivery remains unchanged.

Tests distinguish real payload/executable checks and private RPM database
upgrade/removal using host libraries and --nodeps from a clean Fedora DNF
installation. These unsigned testing RPMs are not official Fedora packages or
complete security qualification. The desktop RSA advisory remains unsuppressed.
