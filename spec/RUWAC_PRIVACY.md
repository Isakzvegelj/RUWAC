# Public repository privacy

Current public content excludes deployment host/account identities, private network
addresses, personal filesystem paths and operational authentication troubleshooting.
Original reports and detailed real-host receipts are preserved outside Git in a
private evidence archive. Public acceptance JSON contains results and scope rather
than raw connection snapshots, process inventories or credential references.

Sanitization covers specifications, release notes, benchmark context, acceptance
JSON, editor placeholders and generated test profiles. A comment-only normalization
in the terminal vendor source is declared in its checksum provenance; runtime
behavior and the authentication implementation are preserved.

Publication uses the existing build/release adapters and controller-owned versions.
The project publication checker scans candidate files and every outgoing commit's
snapshot/message with owner-only external site policy. It fails before allocating a
version if the private policy or standard secret scanner is unavailable. CI runs
public privacy rules, generated scanner tests, full-history secret scanning and
current-source secret checks. The source-release job independently checks current
source and tag metadata before publishing. Raw matches are neither printed nor
uploaded as artifacts. Historical release-note recovery is disabled to prevent
replaying previously removed operational material.

Gitleaks 8.30.1 is checksum pinned. Three reviewed false positives were SHA-256
source-file checksums in two vendored provenance documents. The exception applies
only to the generic-api-key rule, those provenance paths and exact source-checksum
field syntax. Generated service tokens in the same files still fail; ordinary
configuration hashes outside the declared provenance are not exempted.

Validation: 12 generated privacy/scanner regressions, 91 native desktop unit tests,
13 updater guards and 9 GDM-recognition fixtures passed. Workflow syntax and locked
vendor provenance checks passed. These are local checks; hosted runs must be verified
separately. Public results are in [acceptance status](RUWAC_ACCEPTANCE_STATUS.md).

Historical audit: 51 reachable commits and 50 GitHub release descriptions were
inspected. Infrastructure-policy matches remain in historical commits and 24 release
descriptions. No uploaded release assets or workflow artifacts were listed at audit
time. Automatic GitHub source archives still reflect historical tagged trees.
Standard secret scanning found no credential finding after the narrowly reviewed
checksum exceptions; this does not claim an exhaustive absence of secrets.

No historical commits, tags or release descriptions have been changed or deleted.
The private cleanup proposal distinguishes in-place release-description edits from
history rewriting. Existing updaters verify release commits: replacing tags or
rewriting commits requires an approved, coordinated client migration. Current-tree
sanitation does not erase old source archives, clones, forks or caches.
