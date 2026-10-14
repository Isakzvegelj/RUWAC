# RUWAC verified system RPM updates

RPM-installed workspaces now have native installation controls and an explicit
administrator opt-in to automatic upstream testing updates. The system timer is
disabled by default. Updates require exact-source quality/privacy checks, all
installed component assets, hashes, signed workflow provenance and matching RPM
headers before one DNF upgrade. A core-only installation remains headless.

Configuration and staged artifacts are separate from private user profiles.
Opt-out during download prevents installation; already started transactions finish
before disable returns. The GUI refreshes saved state after failed timer operations.
Existing daemons, PTY children, profiles and running RDP windows are preserved.
The root helper accepts only fixed operations, checks installed-file ownership,
uses no owner GitHub credentials, and refuses unsafe locks, directories and logs.

Generated root-update and bridge regressions, genuine signed RPM verification,
disposable Fedora DNF upgrade/rollback/remove, and real GTK worker tests cover the
implementation. Evidence distinguishes cached API/quality replay from live release
checks and real system timer execution. Physical GNOME authorization/input and
external Windows display/clipboard acceptance remain separate gates.

These remain upstream testing RPMs. Workflow provenance is distinct from a signed
Fedora repository or complete security qualification; the RSA advisory is open.
Build, versions and publication remain owned by the existing scripts/controller.

Named terminal sessions now support `ruwac -S NAME`: create or reconnect by exact
name, with ambiguous-name rejection and an interactive-terminal requirement.
Real PTY tests verify creation, rendering, detachment and reconnecting to the same
session. The remote shortcut reuses the existing SSH transport; its end-to-end
remote acceptance remains separate.
