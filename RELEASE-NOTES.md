# RUWAC Fedora source update release

The Updates dialog now offers opt-in automatic installation. A per-user timer checks
eligible GitHub source releases every six hours, compiles locally, preserves installed
components and existing sessions, and retains reversible previous installations.
Automatic installation waits for source quality and privacy checks on the exact
release commit. Opt-out is rechecked before installation, including during a build.
A bootstrap command supports first installation without an existing CLI.

See [installation and updates](spec/RUWAC_UPDATES.md). Source installation is validated
for Fedora 44. This is not a qualified RPM or downloadable binary distribution.
The known desktop RSA advisory remains unsuppressed. Historical privacy disclosures
remain unchanged pending the owner's separately reviewed cleanup decision.
