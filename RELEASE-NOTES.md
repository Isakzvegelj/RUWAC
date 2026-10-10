# RUWAC numeric release-policy correction

Release numbering now uses bare `major.minor.patch`: odd minors are beta and even minors stable. Repeated releases in a lane advance the patch; channel transitions and rollover follow the standard. Historical suffix tags remain intact.

The persistent headless engine and separately compiled Rust GTK4/libadwaita frontend are operational. This source prerelease preserves the product rename, daemon/CLI implementation and native desktop milestone in Git history.

Validation: 15 core unit tests, 11 integration groups and 100 startup races passed. Native GTK/Wayland fixtures passed 17 cards with one PRIMARY card blocked; an isolated GNOME smoke passed five cards. Real cross-process GDK text transfer and PNG staging passed. Benchmarks remain non-idle observations.

PRIMARY, physical desktop gestures, full Screen compatibility, real RDP, agent image attachment and RPM packaging remain incomplete or unqualified. The headless CLI/daemon has no graphical dependencies. This is a source prerelease, not a certified desktop or binary distribution.

See spec/RUWAC_ACCEPTANCE_STATUS.md and spec/RUWAC_INTEGRATION_SPIKE.md for evidence, hashes and limitations. Private launchers, machine paths, original orders and operational configuration are excluded from this repository.
