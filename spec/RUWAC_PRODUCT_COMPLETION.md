# RUWAC product completion audit

## Machine editor, automatic sign-in and remote agent launcher

The machine editor now puts name, host, connection type and service username/password
before the optional GNOME username/password. Credential references and advanced
connection settings are collapsible. Labels sit above fields; narrow dialogs scroll
vertically with Cancel, Save and Save & Connect retained at the bottom. Existing
profile identities, storage and separate Secret Service credentials are preserved.
Save & Connect validates and saves before acquiring the ordinary connection lease.

Automatic GNOME sign-in covers the greeter with a sign-in view while still decoding
and recognizing the authenticated framebuffer. Pointer movement, clicks, scrolling,
keyboard and paste input cannot compete with the automated field sequence. Cancel
or Escape reveals the remote screen; errors and timeouts also return manual control.
Tab/focus/profile/transport changes retain the credential-target cancellation guards.
Dynamic viewport requests wait until automatic sign-in completes or stops.

Operational host details and troubleshooting are retained in private evidence. See [public acceptance status](RUWAC_ACCEPTANCE_STATUS.md) for current qualification and limitations.

Native local RDP tests pass editor CRUD, a 480×520 dialog allocation without horizontal
field clipping, basic/service/GNOME field ordering, cancellation without saving,
Save & Connect for new and edited profiles with authenticated decoded frames, and
input shielding/native cancellation. The remote agent dialog opens and cancels
without creating a session. Its executable bootstrap passes an actual shell/Python
process-boundary test with quoted executable/socket names and both agent plans.
This test uses a disposable CLI peer, not a real external Codex/Claude launch.

Open remote agent appears in Machines. Select a direct SSH endpoint for the Linux
account and Codex/cx or Claude Code/cc. It starts/reuses the remote RUWAC daemon,
creates a persistent session and attaches through an SSH terminal. Remote RUWAC,
Python 3 and the agent must already be installed. Existing cx/cc aliases are used
when available, otherwise the corresponding named application is selected. SSH
uses a key or an interactive terminal password prompt; saved RDP passwords and
Secret Service references are never embedded in the command. Remote provisioning,
complete external launcher acceptance and saved agent contexts remain separate gates.

Real Windows 11 authentication/framebuffer and mixed-workspace join preserved the
transport and PTY. Dynamic resizing did not negotiate Display Control on that tested
bitmap path and remains open; this is not a Windows dynamic-display pass. Authenticated
read-only SMB enumeration confirmed the authorized data share. Windows clipboard,
three external tabs, physical input and qualified Fedora package/distribution gates
remain open. The previous GitHub quality job passed; its dependency-advisory job
failed separately with the existing unsuppressed RSA finding.


## External SSH image consumption and ephemeral Codex verification

Operational host details and troubleshooting are retained in private evidence. See [public acceptance status](RUWAC_ACCEPTANCE_STATUS.md) for current qualification and limitations.


## Automatic RDP clipboard delivery

Same-MIME received copies now reuse an owned GDK content provider and its
selection source. Each consumer read snapshots the current immutable bytes;
updates invalidate RUWAC's in-flight ownership tickets without discarding that
selection. Three-tab tests verify two consecutive text and two consecutive PNG
copies while each pair retains canvas focus. Format changes continue to use
normal GDK selection claims; broader background format-transition/compositor
and clipboard-manager persistence cases remain separate timing gates.

Clipboard-change handling now coalesces one idle refresh per RDP view, after all
GDK clipboard observers have advanced the shared ownership epoch. Only the
current FocusBroker owner updates its bridge epoch and advertises eligible local
formats. A newly focused RDP canvas re-advertises its current local selection,
covering Wayland selection notifications that preceded window activation.
Received providers are still excluded from reflection; Manual, Off and
one-direction policies retain their existing restrictions.

A newer remote format offer can arrive while a clipboard read is pending. The
older result remains ineligible for promotion. After its decoding completes,
RUWAC requests the newest offer only if the current focus, ownership epoch and
policy still authorize automatic receipt. It retains one pending read and the
latest offer metadata, rather than queuing every remote copy or replaying a
cached inactive-session offer.

The released baseline failed the actual authorized Fedora test without Send or
Receive actions. The revised development candidate passed distinct Unicode text
and distinct opaque RGBA PNG transfers both ways at 64×64 and 2560×1440, with
bidirectional policy, automatic saved-account sign-in and authenticated desktop
handover. The local seat/focus driving is isolated and synthetic; the remote
GNOME desktop, CLIPRDR and GDK consumers are real. Owner profiles and existing
sessions were preserved. The portable external driver now exposes `--automatic`
with `--bidirectional`, requires explicit host authorization, and records that
no explicit Send/Receive action was invoked.

The native RDP regression suite adds automatic local-copy delivery across three
loopback tabs, consecutive remote copies without changing canvas focus, and a
bounded delayed-response fixture proving that a newer offer survives an
in-flight read. These generated-desktop fixture results remain distinct from
the real Fedora journey. The normal build/release acceptance runner validates
both against the same frozen compiled artifacts before publication.

Windows clipboard, three external RDP hosts, physical input, broader slow or
malformed-peer timing and qualified Fedora distribution retain their separate
acceptance gates. The full product completion matrix still applies.


## Fedora clipboard ownership and PNG interoperability

The native CLIPRDR bridge advertises both registered PNG names, `PNG` and
`image/png`, and retains the server-assigned ID when receiving either. This
matches GNOME Remote Desktop's MIME-named PNG format without hardcoding its ID.
Both existing DIB formats and Windows Unicode text remain supported.

Receive no longer replays a successfully received remote generation. A fresh
local copy therefore cannot be reclaimed by a redundant Receive action for
that same generation. Disconnect/reset clears the received-generation marker.
Send refuses to reflect the same view's received ContentProvider back to its
server; it asks for a new local copy instead. Optional `RUWAC_RDP_CLIP_TRACE=1`
diagnostics contain event kinds, epochs, format IDs, ownership flags and lengths,
never payload text/images or credentials.

Physical Super presses are deferred until a delivered remote chord or a focused
intentional tap. Compositor-consumed window switching clears the unsent modifier
on focus loss, avoiding an unintended remote GNOME overview. Explicit Send
Super remains available. Three modifier-state units cover a focused tap/chord,
focus-loss cancellation and an inactive release; physical-seat qualification
remains separate.

On the authorized Fedora GNOME host, the production candidate completed automatic
saved-account sign-in, authenticated desktop handovers, and distinct Unicode
text and opaque RGBA PNG transfers in both directions. Directional policies and
bidirectional policy each passed with 64×64 and 2560×1440 PNGs, independent pixel
hashes and different content in each direction. This journey includes explicit
Send/Receive actions; it does not qualify every automatic clipboard timing case.
The local GTK/Wayland seat and focus driving were isolated and synthetic. Only
generated payloads were used; owner profile bytes and existing daemons/sessions
were preserved. Earlier stale-provider failures remain in private evidence and
were followed by clean successful candidate reruns. Windows text/PNG, three
external RDP tabs, physical input and broader clipboard timing/soak remain open.

`ruwac/tests/integration/rdp_external_clipboard.py` is the portable explicit-opt-in
transfer driver. It requires an already authenticated private GUI fixture,
isolated compositor environment, authorized SSH alias and owned remote GDK peer
with an initially empty clipboard. It enforces distinct directional hashes and
preserves generated metadata receipts. It neither provisions secrets nor starts
an owner desktop. The existing loopback RDP suite adds a regression proving that
a repeated Receive leaves a newer local copy intact. Real external and generated
loopback results are recorded separately by the build acceptance runner.

The tested implementation uses the ordinary `build.sh --release --test --desktop`
and `release.sh stable` workflow; controller-owned source releases do not qualify
Fedora RPM/signing or whole-product completion. GNOME's original format naming
is documented in its [clipboard implementation](https://github.com/GNOME/gnome-remote-desktop/blob/50.2/src/grd-clipboard-rdp.c).


## Named workspace lifecycle

The running native desktop now supports `workspace save --name NAME`, `workspace saved`, and `workspace open|restore --id workspace_ID` through authenticated GUI IPC. No GUI or daemon is started by these operations. The existing workspace-control switch explicitly authorizes lifecycle and focus mutations; saved summaries are read-only and omit profile data and credential references. Snapshots are immutable owned 0600 files under a private 0700 daemon-bound catalog, capped at 64 snapshots and 1 MiB per snapshot. Caller-supplied paths, layout payloads, shell commands and credentials are not accepted.

Open adds missing complete saved tabs and preserves current selection; partly open tabs return a conflict requiring explicit Restore. Restore reuses live view objects, reconstructs saved split layouts/tab identities, reattaches missing views to existing terminal sessions and preserves additional views. It does not restart terminal processes or initiate RDP/SSH connections. Missing sessions/profiles remain visible ended placeholders. A live terminal session opened with another view identity produces a conflict rather than replacing that live view; resolving semantic view aliases remains a refinement.

Actual isolated GTK/PTY tests pass mixed terminal/RDP/Files restoration, preservation of additional views and PIDs, closed-view reattachment, private summaries, GUI-instance idempotent save/replay/conflict, permission boundaries and expired-request refusal. RDP pixels in this lifecycle fixture are synthetic; a separate real authorized Fedora test now verifies five restorations preserve the same authenticated transport and receive five newly decoded matching display responses. Windows restoration, physical input and cross-process RDP transport transfer remain separate acceptance gates. Three filesystem tests cover atomic snapshot storage/daemon binding, expired/revoked publication, and symlink/FIFO/hard-link rejection. Storage and daemon lookup run outside GTK, with eight preparation workers, two-second request deadlines and a 128-entry GUI-instance idempotency cache. Other GUI methods retain their existing idempotency contract.

## Verified SSH PNG attachment

`ruwac --host SSH_HOST --socket REMOTE_SOCKET image-attach pane_ID LOCAL_PNG REMOTE_PRIVATE_DIRECTORY` transfers decoded/validated PNG bytes through the existing authenticated OpenSSH stdin channel. Host-key checks remain strict; credentials and agent forwarding are disabled. The image is outside the 1 MiB daemon JSON control frame. A bounded metadata header carries the explicit pane, initial foreground PID, destination, hash, dimensions and byte count. The remote receiver validates those fields and the PNG before sending its remote file path to the selected Codex draft. Source/destination receipts correlate; unexpected hashes, paths, modes, dimensions and fake consumption claims are rejected. No Enter or automatic turn submission is sent. Older remote daemons are preserved and receive an actionable compatibility error.

Operational host details and troubleshooting are retained in private evidence. See [public acceptance status](RUWAC_ACCEPTANCE_STATUS.md) for current qualification and limitations.

Nine binary receiver refusal cases cover oversized metadata/payload, unknown fields, truncation, trailing bytes, hash/dimension mismatch, invalid target and nonprivate staging. Nine synthetic SSH receipt cases cover correlation/version, hash/mode/path/type, Unicode filename and fake-consumption rejection; these are distinct from the real-host test. Native staging now reserves space for both source and final PNG copies, enforcing the 256 MiB peak namespace bound. Uncertain queued files are retained for inspection; no blind retry is performed.

## Native clipboard-to-Codex attachment

The terminal toolbar now offers **Attach PNG to Codex**. It reads actual GDK `image/png`, validates/decode-stages an owned 0600 file, binds delivery to the selected/focused terminal and foreground Codex process, and observes the current draft’s image marker. It never submits a turn. Old conversation markers and model output cannot acknowledge a new attachment. The current persistent engine is preserved: a checked fallback uses its existing authenticated input channel when the new attachment API is unavailable. No local path is offered to an SSH foreground process as a remote attachment.

Operational host details and troubleshooting are retained in private evidence. See [public acceptance status](RUWAC_ACCEPTANCE_STATUS.md) for current qualification and limitations.

Native image staging is bounded to 128 files and 256 MiB per frontend namespace, with 64 MiB input/decoded-image and 4096-pixel dimension caps. Agent files survive frontend close. Retained namespaces expire after 24 hours from their most recent attachment; active-frontends check expiry each minute and later startup reclaims expired closed namespaces. Live transfer files and unknown entries are preserved. Four filesystem tests cover frontend-close retention, expiry, symlink refusal and nonblocking FIFO refusal. Process traversal is globally bounded to 64 entries. Errors retain uncertain queued files and advise inspecting the draft before retrying; already transmitted input cannot be recalled.

This qualifies the supported local Codex image journey. External SSH image consumption and other agent adapters/models, physical input, external RDP clipboard and qualified Fedora packages remain open. Publication uses the existing build/release scripts and controller-managed numbering.

## Automatic GNOME sign-in — compiled package 0.2.6

Machines → Edit → GNOME Remote Login → GNOME User Login now offers **Sign in automatically to the saved Linux account**. This optional endpoint setting preserves existing profiles and defaults off. RDP Service Login still authenticates the transport; the independently stored Linux username/password belongs only to GDM. Windows and Desktop Sharing retain their existing single-stage flow.

The native frontend recognizes supported English Fedora GDM forms from its canonical authenticated remote framebuffer. It selects Not listed, submits the saved Linux username, recognizes the focused Password entry, retrieves the separate Linux secret from Secret Service and sends paired native RDP Unicode/Return events. No clipboard, credential CLI arguments or automation response carries the password. Each action is bound to the current saved profile, endpoint/trust, connection, display generation, selected tab, window/canvas focus and input epoch. Unknown/ambiguous forms do not authorize credentials. User input, focus/tab/profile changes, reconnects and unsupported recognition stop delivery. One password attempt is allowed; missing credentials are refused, and no second attempt is inferred from an unchanged GDM prompt. Already transmitted input cannot be recalled.

Real authorized Fedora 44 acceptance passes one-Connect automatic Linux account selection and password submission, a subsequent authenticated same-endpoint handover and rendering of the existing Linux desktop. The local GTK/Wayland seat is isolated and synthetic; the remote service, GDM and Linux desktop are real. Clipboard is disabled, the SSH-attested original TLS leaf remains mandatory, and remote configuration/owner profiles/existing Linux sessions are preserved. Dynamic resolution also follows the authenticated user-desktop handover and converges to the real requested viewport. Stamped probe duration: 11.724 seconds, including process startup and connection establishment. Real GDM tab switching cancels before password submission; a missing Linux credential stops without a password or desktop handover. Native checkbox off/on persistence preserves both credential references and endpoint identities.

All 25 frozen quality/native/file/RDP checks and 66 desktop units pass. Nine local recognition guards exercise missing OCR, unknown/ambiguous fields, insufficient confidence, malformed data, unsupported dimensions and deadline/child cleanup. Existing dual-credential tests retain wrong service credentials, isolated keyring-unavailable cases, stale profile/focus/reconnect refusal and Windows behavior. Wrong real GDM passwords were not deliberately submitted to the owner account; that acceptance gate remains open. CLI SHA-256: `b86eee086bb1979e8d7fd6b46d189872f262d0c2d1da327639146989e57b5f28`. GTK SHA-256: `8af9bdc7b3bcef79bebf0a3bcb61ba0ca764493444f4bb2bcf5c70779423e081`. Exact commands: existing `build.sh --release --test --desktop` and controller-managed `release.sh stable`, with the owned native acceptance runner.

This is targeted client input automation, not a new GDM/RDP authentication API or system-wide autologin. Automatic background-tab login, other GDM layouts/locales, physical-seat input, external clipboard, unfixed RSA dependency security and qualified Fedora RPM/binary distribution remain separate gates. Automatic sign-in requires local Python/Pillow, Tesseract and English language data; unsupported/missing recognition stops safely. No remote Fedora settings were changed.

## Previous graphics/handover baseline, before automatic sign-in

GNOME modes now use the pinned upstream non-AVC EGFX client/compositor and ClearCodec, Planar and RemoteFX Progressive decoding. Standard/Windows connections preserve the existing rendering path. GFX output is applied to the canonical pointer-aware framebuffer. Graphics reset invalidates stale assisted-input generations. The headless daemon retains no graphical dependencies.

The initial service login uses the existing saved username/password and CredSSP. Enhanced GNOME redirection is parsed with bounded fields; only encrypted, same-endpoint handover is accepted. The next connection negotiates RDSTLS using opaque server-issued credentials, with both original endpoint trust and the previously authenticated TLS leaf required. Supplied destinations/certificates are never adopted. Routing tokens, encrypted credentials and reconnect cookies are excluded from diagnostics/persistence. This internal service handover is independent of the Linux user credential at GDM.

Actual authorized Fedora 44 integration renders GDM after service authentication and handover. Twelve real resolution changes and three new-transport reconnects pass, with synthetic local GTK controls on an isolated compositor. No Linux password or keyboard input was sent; the probe disabled clipboard. Linux desktop sign-in, external GNOME text/image clipboard, physical input and automatic second-stage login remain unverified. Existing assisted field delivery still requires explicit selected-tab/endpoint/login-screen/focus confirmation. Remote GNOME/GDM configuration was unchanged.

Upstream origin: IronRDP commit `e4143f26306a6639a889d80c1c484ca69c365a8f` for EGFX/graphics, registry session 0.11.0 and connector 0.10.0 at `11a0810cfbbabd8b8023875a05e3041216d4b01b`; MIT OR Apache-2.0, with retained notices and per-file provenance. Compatibility adapters preserve the existing core/PDU APIs. AVC420/444 are disabled; no H.264 capability is advertised. Existing RSA advisory remains visible and unfixed. Qualified Fedora RPM/signing and physical-desktop acceptance remain open.

The full product is not yet complete. The stable source lane contains operational compiled code and exact regression evidence; it does not qualify the entire product or a Fedora binary distribution. Authorized Windows authentication/framebuffer decoding passes; Fedora GNOME service authentication, RDSTLS handover and actual GDM rendering are verified; Linux-user sign-in remains separate. Physical input, dependency security and the remaining terminal, files, agents, automation and distribution scope remain open.

Operational host details and troubleshooting are retained in private evidence. See [public acceptance status](RUWAC_ACCEPTANCE_STATUS.md) for current qualification and limitations.

Native endpoint-bound certificate review/pinning and changed/stale guards have four policy units and eight authenticated local native groups. Real Windows authentication/framebuffer decoding passes with an existing approved leaf; the Fedora SSH-attested leaf is verified and GNOME service authentication/handover/GDM rendering passes. Owner clipboard/keyring/session data are never synthetic fixtures; authorized external credential lookup is read-only. Optional browser/agent adapters and later macOS builds remain staged scope.


Authorized Fedora SSH headless/real remote-PTY acceptance passed on 2026-10-11. Mixed terminal/RDP/Files checkpoint restoration and normal application activation are implemented; detailed remaining view-management gates apply. Every remaining full-product gate above continues to apply.


Owner requires automatic GNOME account selection and sign-in on Connect. The supported English Fedora GDM path now passes on a real authorized host. Universal/unattended background login remains unqualified; assisted delivery remains available for unsupported screens.


## Native remote Codex image route

Attach PNG to remote Codex… uses an explicit registered SSH profile, absolute remote daemon socket, stable pane ID and existing private staging directory. Endpoint ID, resolved account/address and explicit confirmation bind the reviewed destination. Profile edits require a fresh review; focus/tab/local-pane/profile changes cancel ongoing delivery. Passwords and key passphrases use the existing Secret Service/libssh2 components, strict known-host verification and selected private key; unsupported gateways are refused. The shared binary PNG sender/receiver retains hash/dimensions/mode/path/correlation and remote foreground guards. Four remote workers and per-read/transfer deadlines bound work outside GTK; the local daemon worker continues serving input/rendering. Local staging reserves three copies for this route. No automatic submission or retry occurs.

Operational host details and troubleshooting are retained in private evidence. See [public acceptance status](RUWAC_ACCEPTANCE_STATUS.md) for current qualification and limitations.


## Remote Codex pane discovery

Find remote Codex panes reads all bounded job pages over the reviewed SSH profile and explicit daemon socket. On current engines, the read-only agent.probe method reuses the same-UID bounded process-group/child check used by image delivery, so a Node launcher is recognized without argv/environment/conversation inspection. It reports process presence, foreground PID and selected state only; it does not infer the model, authentication, readiness or consumption. Older engines retain exact executable-name suggestions and manual pane entry. No daemon/session is started or replaced.

The native list excludes ordinary/ended processes and displays session/window names, PID and stable pane ID. Selection resets confirmation and binds the foreground PID; delivery rejects a changed foreground process before transfer. Profile/socket/window changes cancel in-flight discovery, and stale completed rows refuse selection. Discovery and delivery share the four-worker cap; paged membership/cursor/count/identity guards are reused and the list scrolls within a bounded dialog area. Actual owned Wayland/GDK, generated-key authenticated loopback SSH and provider consumption pass the complete discovered-pane journey. External-host consumption, saved destination contexts and other agent adapters retain their gates.


## Bounded GUI metadata event feed

Authenticated subscribe/poll/unsubscribe now cover coalesced tab/focus/layout/RDP state observations, with four typed filters, monotonic per-instance cursors, actor UID binding,64-subscription and256-event bounds,60-second idle expiry and explicit overflow/resynchronization. Metadata reads do not enable focus/lifecycle/input permissions. Four actual GTK/PTY/authenticated-loopback RDP groups plus component actor/filter/expiry/capacity/gap/instance tests pass. Local controls are synthetic; no physical or external-host pass is implied. The documented feed polls100ms observations and excludes clipboard/terminal/credential/agent content. Exact-transition journaling, push streaming and remote input remain separate scope.

## Clean Fedora RPM lifecycle and launcher compatibility

Actual published Fedora 44 core/desktop RPMs pass DNF installation, matching-pair
upgrade, downgrade to a genuine prior source-release build and removal in a fresh
pinned Fedora filesystem. GTK/libadwaita, image and OCR runtime dependencies are
resolved by DNF; --nodeps is not used. The same daemon and raw-receiver PTY survive
upgrade and rollback, and generated private configuration remains unchanged.
The bounded RAM filesystem and owned containers are removed after testing; the
host package database, user homes, desktop and RDP sessions are never mounted.
This is clean-container dependency/lifecycle evidence, not physical laptop GNOME,
RPM repository signing or complete security qualification.

The reproducible opt-in driver is ruwac/tests/integration/rpm_dnf_lifecycle.py.
It requires a cached pinned Fedora image, matching current/prior RPM pairs and
explicit --run-owned-fixture; its RAM filesystem and container each have a 2 GiB
limit. Preserve its private receipts outside the source tree.

The remote Codex launcher first preserves an existing cx alias/function. Otherwise,
a read-only help probe checks whether the installed CLI supports the agents browser.
Supported versions open that browser without a prompt; older versions open the
ordinary TUI without a positional message. Process-boundary tests verify both
variants, user-command precedence, unavailable Codex and quoted RUWAC/socket names.
Read-only help checks on authorized systems do not create interactive agent threads.
The complete external interactive launcher remains a separate acceptance gate.
