# Security policy

RUWAC currently publishes engineering prereleases. The latest source prerelease is the maintained development baseline; older releases remain history. No production-stable support window or response-time guarantee is established.

## Report a vulnerability

Use the repository's [Security page](https://github.com/w3fans/RUWAC/security) and **Report a vulnerability** when private reporting is available. If the button is unavailable, open an issue requesting a private security contact without including vulnerability details, secrets or personal data. The maintainer account is [w3fans](https://github.com/w3fans). Private reporting availability is a repository setting, separate from this policy. See [GitHub's reporting instructions](https://docs.github.com/en/code-security/how-tos/report-and-fix-vulnerabilities/report-privately).

A private report should identify the affected release/commit, component, prerequisites, impact and a minimal reproduction using generated data. Include sanitized evidence and proposed mitigations when available. Do not include passwords, private keys, live clipboard contents, endpoint credentials or raw operational logs.

Test only systems and accounts you are authorized to use. Preserve active terminals and connections. Use isolated credentials, sockets and profiles; reproduce destructive behavior against disposable data.

## Maintainer handling

Maintainers assess reports privately, reproduce against isolated fixtures, prepare reviewed fixes and coordinate disclosure with the reporter. Security-sensitive patches still require testing and the standard release authority. A public advisory or release notice should explain affected versions, impact, mitigation and the verified fixed reference. A source tag alone does not prove qualified binary/package distribution.

## Current boundaries

The control protocol uses private Unix sockets and same-UID peer checks; it is not a boundary against another process already acting as the same user. RDP and SSH trust must be verified explicitly. Credentials use Secret Service references rather than plaintext profile fields. Full external interoperability, adversarial protocol/clipboard soak, hardware acceptance and Fedora packaging qualification remain open in the product acceptance audit.

## Known dependency findings

The desktop lockfile includes `rsa 0.10.0-rc.18` through IronRDP's `picky`/`sspi` authentication dependencies. [RUSTSEC-2023-0071](https://rustsec.org/advisories/RUSTSEC-2023-0071.html) reports private-key recovery through network-observable timing and lists no patched version. The dependency scan remains failing and no advisory exception is configured. This is a stable-security qualification blocker; a source prerelease is not a claim that it is resolved. Current client configuration uses explicit username/password credentials; that observation is not proof that all affected dependency paths are unreachable. Broader authentication-provider reachability analysis and an upstream fix or reviewed alternative remain necessary.

The scan also reports unmaintained `atomic-polyfill` and `rustls-pemfile` dependencies. Their presence is recorded for dependency maintenance; it is not treated as a clean advisory scan. The headless lockfile currently has no reported findings against the inspected advisory snapshot. New snapshots may change these results.


Public infrastructure privacy is separate from secret scanning. Publication checks
scan every candidate file and outgoing commit snapshot/message; deployment-specific
policy is supplied through an owner-only file outside Git. CI runs the public rules
and checksum-pinned Gitleaks against the current source. Scanner regression examples
are generated at runtime, never copied from owner infrastructure. Raw matches and
operational receipts must not be uploaded as CI artifacts or placed in issue reports.
Historical disclosures require a separate, privately reviewed cleanup decision;
cleaning the current tree does not remove old tags, clones or release descriptions.
