# Fedora testing bundles

Fedora RPM assembly fixes the build time to the source commit timestamp and
clamps packaged file timestamps to that time. Before publication, the release
workflow assembles the same compiled inputs again in a different directory and
requires byte-identical complete core and desktop RPMs. This checks deterministic
packaging; it does not prove reproducible compilation from source or qualification
for a signed Fedora repository.

The product-owned bundle assembler packages the existing native desktop and
headless CLI without assigning versions. The existing tagged GitHub workflow attaches checked
archives and SHA256 sidecars to its controller-created release; the assembler itself
does not publish. Source releases
continue through the existing build/release adapters. It requires a clean source
commit matching its existing controller-owned tag and matching compiled versions.

Compile with source-path remapping before assembly. Rust's
[path-remapping option](https://doc.rust-lang.org/rustc/command-line-arguments.html#--remap-path-prefix-remap-source-paths-in-output)
rewrites emitted source paths; native compiler prefix maps cover native code.
Maintainers supply private root mappings locally. Never publish those roots or
reuse artifacts that fail the complete bundle privacy check.

```sh
python3 scripts/bundle-user.py --build-root "$RUWAC_BUILD_ROOT" --output "$RUWAC_BUNDLE_OUTPUT" --require-private
```

The private site policy and checksum-verified standard secret scanner must be
available to the maintainer. The helper never embeds the policy or raw matches.
ELF data, constants and symbols are checked separately from instruction bytes;
short opcode coincidences are not reported as textual infrastructure identifiers.
ELF relocation records and pointer-sized fields identified by x86_64 relocations
are treated as ABI numeric data. Actual constants and string tables retain short
identifier checks; generated regressions distinguish pointer bytes from literal
three-letter strings. Invalid relocation tables fail parsing.
Long private values and complete private-key material are also checked across the
whole artifact. Crypto-parser BEGIN markers alone are not private keys. Printable
data sections are additionally passed to the standard secret scanner. Source
publication retains its existing stricter marker checks.
One exact public libssh2 diagnostic is recognized by its full string hash when
the generic API-key rule mistakes its error wording for a key. Other findings,
changed strings and all other rules still fail. No blanket artifact exclusion or
dependency-advisory suppression is used.

A public upstream copyright name can coincide with a private policy term. Preserve
the original legal text. A privately reviewed exact document hash may be supplied
through `RUWAC_PRIVATE_NOTICE_ALLOWLIST`, outside source with owner-only permissions.
Only that unchanged notice's private-policy finding is excepted; other privacy
rules, modified notices and ordinary source files still fail. No policy contents,
matching values or operational evidence enter the archive.

The output is a normalized `.tar.gz` archive and a SHA256 sidecar. Archive creation
is deterministic for fixed inputs, atomic, and refuses to overwrite existing
outputs. This does not establish reproducible compilation or a signing chain.

Dependency documents cover the conservative Linux runtime/build dependency closure
for both applications, excluding development-only and other-target edges. Original
license/notice texts and hashes are retained; package-index rows contain no local
source paths. Registry documents are checked against extracted-archive checksums.
Where a registry archive omits its license, the supplement is pinned to that
archive's recorded VCS commit with a checked document hash. Missing, altered or
stale documents fail assembly. Declarations and preserved documents do not replace
a complete legal or security review.

Copy an approved testing archive and its trusted checksum reference to the target
Fedora 44 x86_64 machine. Verify the external checksum before extracting. Install
runtime dependencies and run the included installer from the extracted directory:

```sh
sudo dnf install gtk4 libadwaita openssl-libs libssh2 sqlite-libs python3 python3-pillow tesseract tesseract-langpack-eng
python3 install-user.py --build-root . --desktop
"$HOME/.local/bin/ruwac-desktop"
```

The installer validates every bundled file before executing either binary, rejects
symlinks and unexpected files, and checks the OS/architecture and artifact versions.
Installation itself requires no root access or Git checkout. It retains notices
with immutable releases and preserves existing profiles, credentials, daemon
sessions and running executable inodes. It refuses an existing same-version
installation with different artifacts. Use a later controller-owned release for
a changed build; do not overwrite that installation.

GitHub release downloads now include the Fedora testing archive and its SHA256
sidecar once the tag's build and exact-commit quality/privacy gates complete. The
archive metadata includes a [GitHub asset SHA256 digest](https://docs.github.com/en/rest/releases/assets).
The workflow preserves existing same-name assets; differing bytes require a later
controller release, not replacement. Tags and old release notes remain unchanged.

Source-build updates remain the default. To select checked testing bundles and
opt into unattended installation on Fedora 44 x86_64:

```sh
"$HOME/.local/bin/ruwac" update --testing-bundles on
"$HOME/.local/bin/ruwac" update --auto-install on
```

The equivalent checkbox is in Updates. Runtime prerequisites above are sufficient
for bundle delivery; development prerequisites apply to source-build delivery.
Use `--testing-bundles off` to restore source builds, or `--auto-install off` to stop
unattended installation. Existing rollback also disables automatic installation.
A missing or unchecked asset leaves a pending update and does not compile silently.

The updater verifies the expected repository/tag/asset name and HTTPS origins,
GitHub's SHA256 digest and size, download/decompression budgets, regular-only unique
archive members, the exact source commit and every manifest file. Changed settings
or a newer installation discovered during download prevent the installation.
These checks establish integrity under the repository/GitHub trust model; they do
not provide independent publisher signing or attestation.

Testing bundles retain the known desktop RUSTSEC-2023-0071 advisory explicitly.
RPM/signing, publisher attestation, clean-target interoperability and physical
input remain separate acceptance gates.


## Build provenance

Hosted testing archives include a signed `.sigstore.json` attestation bundle,
generated by the official commit-pinned GitHub action after exact-commit quality
and privacy checks. The publisher verifies the repository, signer workflow, source
ref/commit and hosted runner before attaching the archive. Offline verification
uses the downloaded archive and signed bundle:

```sh
gh attestation verify "$ARCHIVE" --bundle "$ARCHIVE.sigstore.json" --repo w3fans/RUWAC --signer-workflow w3fans/RUWAC/.github/workflows/release.yml --source-ref "$RELEASE_REF" --source-digest "$SOURCE_COMMIT" --deny-self-hosted-runners
```

Obtain the expected ref and commit independently from the controller's source
release. A checksum and a valid signature over a different source are insufficient.
The updater's existing digest/manifest verification remains separate; it does not
silently claim attestation enforcement. See the [official verification policy](https://cli.github.com/manual/gh_attestation_verify).
This provenance is a build-origin claim and does not establish reproducible builds,
code safety, RSA remediation or qualified RPM signing.
