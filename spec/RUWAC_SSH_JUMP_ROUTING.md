# Explicit SSH jump routing

Set **SSH jump / bastion alias** in Advanced connection settings to an existing
OpenSSH configuration alias. Leave it empty for direct connections. Network
groups can supply the alias when connection inheritance is enabled; an explicit
endpoint alias takes precedence. RDP does not inherit SSH routing.

A generated example for a disposable test configuration:

```sshconfig
Host generated-bastion
    HostName bastion.example.invalid
    User generated-user
    IdentityFile ~/.ssh/generated-bastion
    IdentitiesOnly yes
```

Verify and enroll the gateway host key independently before using the route.
Gateway authentication uses its configuration key or agent in batch mode, with
strict host-key verification. Password prompts and the target's saved password
are not used for gateway authentication. Existing Gateway metadata is descriptive
and does not enable routing.

Native SFTP and resume open an anonymous socket pair to an owned `ssh -W` process,
then authenticate and verify the target using the existing libssh2 adapter.
Rsync, SSH terminal and remote-agent launch use a quoted fixed ProxyCommand for
the same alias. The gateway helper disables agent forwarding and shared control
masters. New terminal/agent launches use a separate private target control socket
for implicit image channels; they reuse the already authenticated route, including
its jump, and refuse network fallback if the original SSH process is unavailable.
Remote-agent prerequisites remain unchanged; this adds the route, not an agent
installation or external agent acceptance result.

Existing direct-transfer recovery hashes are unchanged. Routed jobs bind the jump
alias; editing that alias refuses resume instead of silently changing routes.
Existing JSON and SQLite profile migration paths preserve an empty default and
omit the new field when unused. Older readers with strict preference decoding
cannot load profiles that use the new field; retain a private backup before a
downgrade. Remmina import remains read-only and cannot import routing credentials.

Earlier jump-routing qualification: 105 desktop unit tests; eight actual two-server SSH groups (SFTP
upload/download, independent trust, independent authentication, resume route
binding, active cancellation, recursive rsync, terminal command and owned-process
cleanup); four native GTK editor callbacks on an isolated Wayland compositor.
The fixture-only executable shim selects a disposable SSH config without changing
an owner's home configuration. These are generated local fixtures, not external
bastion or physical-input acceptance.

Reproduce after building the fixture driver into your build directory:

```sh
cargo build --locked --manifest-path ruwac/desktop/Cargo.toml --example files-fixture
python3 ruwac/tests/integration/ssh_jump.py --driver "$BUILD_ROOT/target/debug/examples/files-fixture" --evidence "$PRIVATE_EVIDENCE"
```

The test needs OpenSSH server/client, rsync 3.4 or later, and a non-root test account
with a private runtime directory. It creates its own loopback servers, trust and
keys; generated receipts and keys stay outside the publication inventory.

For implicit PNG delivery, the remote receiver reads the declared byte count and
verifies its hash before sending readiness. Publication then requires the local
uploader's commit while its cancellation ticket is valid. Cancelling before that
commit removes the staged file without ending the terminal's SSH master. These
rules do not change file-transfer recovery or infer effective routes for other
applications. Broader external gateway and provider acceptance remains separate.

Protected PNG delivery can also be reproduced against generated direct servers
and the explicit jump alias after building the release fixture adapter:

```sh
python3 ruwac/tests/integration/ssh_protected_upload.py --driver "$BUILD_ROOT/target/release/examples/files-fixture" --evidence "$PRIVATE_EVIDENCE/protected-direct"
python3 ruwac/tests/integration/ssh_protected_upload.py --driver "$BUILD_ROOT/target/release/examples/files-fixture" --evidence "$PRIVATE_EVIDENCE/protected-jump" --jump
```

These checks need Python Pillow and a non-root test account with OpenSSH client
and server tools. Use an owned bounded test environment. They leave owner SSH
configuration unchanged and retain generated evidence outside the checkout.
