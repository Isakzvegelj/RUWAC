# Fedora testing bundles

The product-owned bundle assembler packages the existing native desktop and
headless CLI without assigning versions or publishing artifacts. Source releases
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

Automatic updates continue compiling published source. Install the development
prerequisites in [RUWAC_UPDATES.md](RUWAC_UPDATES.md) before opting in. A testing
bundle is not a signed RPM or a qualified stable security distribution. The known
desktop RUSTSEC-2023-0071 advisory remains explicit. RPM/signing, publisher
attestation, clean-target interoperability and physical desktop input remain
separate acceptance gates; bundle assembly does not claim they pass.
