# RUWAC source release

RUWAC is a compiled Rust terminal daemon/CLI and an optional native GTK4/libadwaita desktop with mixed terminal, embedded RDP and SSH/SFTP workspaces. Existing terminals and connections remain independent of desktop presentation. The desktop uses a machine sidebar, tabs and visible connection/display controls inspired by Remmina, with Alt+1–0 tab switching.

This increment repairs hosted GitHub workflow validation. The previous source prerelease's quality workflow was rejected before jobs started because runner.temp was used in job-level environment declarations. Temporary build/scanner paths now initialize in steps through GITHUB_ENV. Checksum-pinned actionlint validates expression context availability locally and in CI; it reproduced the four original errors and passes the repair.

Contributor/security/conduct policies, CODEOWNERS, issue/pull-request templates, pinned Fedora source checks and guarded contribution labels remain available. The read-only quality workflow checks formatting, Clippy, unit and actual headless fixtures, plus locked declaration/checksum/headless-dependency boundaries. Contribution labels use only trusted base code and preserve existing labels.

Local and isolated pinned-Fedora tests validate the checks and synthetic label guards; complete functional regressions repeat on the stamped build. Missing OpenSSL CLI in the initial clean-container run was corrected in declared CI prerequisites. Live hosted workflow and actual PR label delivery remain distinct from local container/API-fixture evidence.

The new advisory scan reports RUSTSEC-2023-0071 in the desktop's transitive rsa 0.10.0-rc.18 authentication dependency, with no patched version listed upstream. It also reports unmaintained atomic-polyfill and rustls-pemfile. The headless snapshot has no reported findings. No advisory exception is configured, and the security job deliberately fails. Stable security qualification remains blocked; source prerelease publication does not resolve the finding. Current username/password client configuration is not proof of complete provider-path unreachability. See SECURITY.md and the acceptance report for exact scope.

The bounded 64 KiB process metadata transport and full 128-pane native monitor remain validated, with stable membership, exact-pane jump and malformed cursor/identity guards.

Persistent SFTP pause/resume/discard, interrupted overwrite recovery and encrypted SSH key passphrases through Secret Service remain available. Their fourteen local groups, six existing SFTP groups and two actual authorized Fedora resume groups repeat in this candidate workflow.

File transfers currently cover individual regular files up to 32 GiB, with 128 pending jobs. Recursive transfer and rsync remain open. Full source/prefix/staged rehashing adds reads. SFTP path review does not provide a filesystem-wide transaction against independent actors, and server close does not establish remote storage power-loss durability.

Sixteen real GNU Screen/tmux/RUWAC chains at two-to-five layers remain qualified in their tested configurations. GNU Screen 5.0.1 still interprets its Control-A detach command inside bracketed pasted payloads; arbitrary-control paste through Screen remains an observed upstream boundary. The explicit leaf RUWAC pane API bypasses intermediate key parsers.

The GitHub workflow verifies source/tag/package parity and creates the source release page. It does not compile binaries. Local build.sh compiles both executables; private per-user installation is available. Publication uses the existing release.sh and version authority. Original source is Apache-2.0, and dependency licenses are preserved.

Stable qualification remains open: authorized Windows/Fedora RDP credentials and trusted endpoint identity, physical GNOME input/PRIMARY/HiDPI, remaining agent/file/inventory/automation features, checkpoint and performance/soak qualification, and qualified Fedora package/binary distribution. RPM packaging currently exits 78 because no qualified app-linux package adapter is installed; the defect is recorded for the release infrastructure owner. Reachable RDP/SMB ports do not prove login or file-share interoperability. Earlier live daemons and graphical clients are preserved during installation; new features apply to newly started matching processes. Checkpoints recover ended history, not running programs; hidden primary history during alternate-screen use remains open.
