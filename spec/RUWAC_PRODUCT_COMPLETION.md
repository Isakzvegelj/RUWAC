# RUWAC product completion audit

## External SSH image consumption and ephemeral Codex verification

After owner reauthentication, an authorized EVF integration passed RUWAC SSH
PNG transport, independent remote SHA-256, private file/directory modes, real
Codex draft attachment and actual generated-image consumption. A separate
`codex exec --ephemeral` run on EVF Codex 0.162.1 correctly read the same generated
image with exit zero and no new persisted session files. It ran noninteractively
in the owned fixture directory with a read-only sandbox, preserving owner
configuration and existing terminal sessions. These are actual external-host
consumer results, distinct from the native registered-SSH route's earlier
loopback acceptance. The complete native GTK→external EVF consumer journey,
saved destination contexts and other provider adapters remain separate gates.
Earlier expired-token results below describe their historical baseline; they
are superseded for this verified external CLI consumer journey.


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

Real authorized CVF→EVF acceptance verifies the PNG hash independently on the remote filesystem, 0600 file/0700 directory modes and acceptance by the real Codex 0.144.1 draft. **Remote image consumption remains blocked:** EVF's saved Codex access token could not be refreshed. Cached login status is insufficient proof of a usable account. No credentials are copied from CVF, no account logout occurs, and attachment is not reported as consumption. The owner has been asked to sign in on EVF. GTK attachment to an ordinary SSH foreground process still refuses; routing the GTK action to a registered remote agent remains an implementation gap.

Nine binary receiver refusal cases cover oversized metadata/payload, unknown fields, truncation, trailing bytes, hash/dimension mismatch, invalid target and nonprivate staging. Nine synthetic SSH receipt cases cover correlation/version, hash/mode/path/type, Unicode filename and fake-consumption rejection; these are distinct from the real-host test. Native staging now reserves space for both source and final PNG copies, enforcing the 256 MiB peak namespace bound. Uncertain queued files are retained for inspection; no blind retry is performed.

## Native clipboard-to-Codex attachment

The terminal toolbar now offers **Attach PNG to Codex**. It reads actual GDK `image/png`, validates/decode-stages an owned 0600 file, binds delivery to the selected/focused terminal and foreground Codex process, and observes the current draft’s image marker. It never submits a turn. Old conversation markers and model output cannot acknowledge a new attachment. The current persistent engine is preserved: a checked fallback uses its existing authenticated input channel when the new attachment API is unavailable. No local path is offered to an SSH foreground process as a remote attachment.

Real acceptance uses a separate Wayland GDK clipboard producer and authenticated Codex 0.162.1 on CVF. The actual compiled toolbar callback accepts the generated PNG, verifies its hash/mode, cancels a slow read on Wayland focus loss and refuses an ordinary terminal. Codex consumes the PNG after the frontend is closed and before the private daemon is stopped. Both the new engine and an isolated immutable 0.1.29 engine pass. Input/focus driving is synthetic on an isolated compositor; physical-seat acceptance remains separate. No owner clipboard, configuration or sessions are used by those fixtures.

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

| Requirement | Current evidence | Status / remaining work |
|---|---|---|
| D-00 independent windows | Actual GTK/shared-daemon window close and independent private layouts | PASS bounded native fixture; first window owns default GUI automation endpoint; RDP transport transfer between processes remains |
| RDP-06 GNOME dual login | Separate credentials, assisted input and opt-in recognized native login | Real authorized Fedora automatic Linux sign-in PASS; tab/missing-secret refusal PASS. Other layouts/locales, background tabs and wrong real GDM password remain open |
| T-01 Screen commands | Nine real-PTY copy/chooser/prompt/configuration and 2–5 RUWAC nesting groups | PARTIAL: implemented bounded headless interaction; mixed GNU Screen/tmux 2–5 layers pass in scoped configurations; Screen arbitrary-control paste and physical input qualification remain |
| T-02 startup input | 100 adversarial probes in headless suite | PASS fixture |
| T-03 multiple PTYs and load | Independent PTYs, layout suite, actual 128-pane process/wire/native paging fixture | PASS bounded capacity; maximum-load terminal/checkpoint performance remains separate |
| T-04 detach and reattach | Native crash/reattach and workspace same-PID tests | PASS daemon lifetime; no process resurrection claim |
| T-05 stable IDs | Targeted routes, stale-ID negatives, concurrent clients | PASS |
| T-06 VT/Unicode/applications | Native Vim/less/htop and actual fullscreen fzf Unicode/Alt navigation, canonical extended SGR/plain+palette recovery and native/Cairo output | PARTIAL: complete VT/application/Kitty/AltGr/IME and physical-input matrix |
| T-07 mouse recovery | Native routing and disposable Bash raw-child exit | PARTIAL: physical touchpad gate blocked |
| T-08 history/internal buffer | Canonical history, paged capture, independent internal/system clipboard tests | PASS bounded fixture |
| T-09 SSH/nested terminals | Real authorized SSH remote-PTY detach/reattach and Unicode | PARTIAL: actual 2–5 mixed GNU Screen/tmux/RUWAC layers pass with both prefix modes; Screen arbitrary-control paste remains an upstream limitation |
| T-10 performance | Retained matched 1/5/20/40-pane measurements | PARTIAL: current 30-trial matched measurements and per-block unrelated-load accounting implemented; observed desktop/IDE load blocks idle qualification |
| A-01 negotiation/discovery | Existing v0 schema/capabilities/typed errors | PARTIAL: describe/local GUI targeting and typed RDP lifecycle over actual SSH implemented; read-only paged host inventory/list/query now passes native and authenticated SSH tests; named workspace save/open/restore now pass scoped GTK/PTY acceptance; native snapshot management and terminal session aliases implemented; real Fedora RDP restoration now passes five cycles with fresh frame responses; Windows restoration and nonterminal alias policies remain |
| A-02 literal input | Unicode, quotes, shell metacharacters and separate Enter | PASS |
| A-03 concurrent routes | Idempotency, focus changes, stale IDs | PASS |
| A-04 non-TTY/SSH | Real --host JSON/file control and three SSH PTY cycles | PASS authorized SSH fixture; endpoint provisioning remains explicit |
| A-05 capture/wait/events | Bounded capture, event subscription, revision and wait tests | PASS |
| A-06 socket/backpressure | Owned daemon socket, malformed frames, concurrency limits | PARTIAL: same-UID peer verification and byte-bounded process metadata/all-page native guards validated; broader response bounds and fuzz/soak remain |
| A-07 GUI control | Explicit unavailable response and independent headless graph | PARTIAL: independent permission-scoped focus/RDP lifecycle over actual SSH validated; bounded polling GUI metadata event subscriptions now pass native/authenticated-loopback acceptance; remote input and push-stream transport remain |
| R-01 Windows | Authorized Windows 11 authentication and real 1280×720 framebuffer decoding | PASS bounded real-host login/frame; external clipboard, display/input and broader interoperability remain |
| R-02 Fedora GNOME | Actual authorized service login, greeter/user-desktop handovers, automatic Linux sign-in and dynamic viewport | PARTIAL: supported English real-host path PASS; physical input, other locales/layouts and external clipboard remain open |
| R-03 independent tabs | Three authenticated loopback tabs; 200 switches/450 inactive offers | PARTIAL: external-host qualification |
| R-04 display modes | Actual Display Control/reactivation, viewport transforms and resize storm | PARTIAL: external server and physical pointer acceptance |
| R-05 remote keys | Scancode mapping and send-key menu | PARTIAL: actual Windows/GNOME layouts, AltGr/IME/compositor restrictions |
| R-06 pointer/touchpad | Synthetic routing, held-input release and reconnect | PARTIAL: physical input and external hosts |
| R-07 HiDPI | Fractional transforms and actual 3K lossless bitmap frames | PARTIAL: real monitor moves/scales and external codec |
| R-08 responsiveness | Frame timings, cadence and process samples in loopback | PARTIAL: matched authorized Remmina/FreeRDP comparison |
| C-01/C-02 text interoperability | 60 bidirectional loopback transfers | BLOCKED: trust/credential profiles configured; external text/image acceptance remains open; Fedora GDM rendering is now implemented |
| C-03/C-04 PNG interoperability | 18 loopback PNG/DIBV5 transfers | BLOCKED: trust/credential profiles configured; external text/image acceptance remains open; Fedora GDM rendering is now implemented |
| C-05 ownership | Real GDK peer and three loopback RDP tabs; Files isolation | PASS owned compositor |
| C-06 PRIMARY/internal separation | Independent internal/CLIPBOARD tests | PARTIAL: PRIMARY protocol and physical middle-click acceptance |
| C-07 application paste | Real GDK peer, native applications, 10 KiB/1 MiB payloads | PASS owned compositor |
| C-08 local agent image | Real native GDK → Codex draft and actual consumption, including frontend close and preserved legacy engine | PASS scoped local Codex 0.162.1; synthetic local input, physical-seat gate separate |
| C-09 remote agent image | Real authenticated SSH PNG transport, independent remote hash/modes and Codex draft acceptance | PARTIAL: actual external EVF CLI transfer/draft/consumption now passes after owner reauthentication; ephemeral noninteractive consumption creates no persisted session files. Native registered SSH routing passes real authenticated loopback transfer/consumption; the complete native external-host journey remains separate |
| C-10 transactions/errors | Epoch cancellation, policies, reconnect and limits | PASS bounded fixture; further malformed/slow-server soak remains |
| C-11 unsupported format/consumer | Visible staging errors and no fake attachment claim | PARTIAL: final destination/consumer fallback UX |
| F-01 import | Explicit read-only Remmina preview, 70 imports, metadata backup | PARTIAL: explicit-ID reviewed merge, policy preservation, stale/active guards and Undo validated; broader nonsecret mapping/skip reporting remains |
| F-02 library | Native compact sidebar/panels, CRUD, groups, favorites, recents, 1,000-profile fixture | PARTIAL: native 50-row paging and all 1,000 endpoints/search validated; transactional canonical SQLite inventory and legacy recovery validated; inherited routes remain |
| F-03 transfers | SFTP SHA-256, native pause/resume/discard, original-host binding, lost-rename recovery and encrypted keys through Secret Service; 14 new and six existing fixture groups | PARTIAL: native rsync file/folder copies, actual authorized SSH integrity and cancellation pass; client SIGKILL recovery and native/external verified backup handling pass; partial-data resume, broader implementations, remaining independent-actor races and remote power-loss durability remain |
| F-04 agents | Ordinary terminal execution, bounded process metadata, native search/exact jump and private D-Bus exit delivery | PARTIAL: observed process layer validated; SSH image consumption, other agent adapters, physical notification acceptance and optional structured adapters remain |
| F-05 ordinary user/headless package | Independent graphical-free headless graph | PARTIAL: per-user installation implemented; RPM dependency/install qualification remains open |
| F-06 install/upgrade/uninstall | Reversible private per-user installer and desktop entry | PARTIAL: per-user installer fixtures pass; Fedora RPM lifecycle remains open |
| F-07 standard workflow | Existing build/release/controller and verified tagged source releases | PARTIAL: immutable binary/RPM publication remains unqualified |
| F-08 public hygiene | Explicit permitted source inventory, outgoing-history checks | PASS for verified source publications |
| Mixed workspace | Live mixed GTK trees and all-view checkpoints/crash restore | PASS bounded native fixture: persistent tab/view IDs, exact leaf focus, move/close and crash restoration; external-host/physical qualification remains separate |
| Persistent metadata/history after daemon loss | Real SIGKILL and runtime-tree removal restore stable IDs, canonical active-screen tail/cells, layout and ended status; native copy passes | PARTIAL: 128 dense 100 by 200 dual-buffer PTYs checkpoint below the unchanged limit and preserve recorded cell styles/text after SIGKILL under a verified 1536 MiB service cap; full-scrollback saturation, arbitrary entropy and idle latency qualification remain |
| Native accessibility and diagnostics | GTK controls, configurable keys and accessible visible RDP toolbar | PARTIAL: global Alt-number navigation and safe targeted doctor implemented; physical keyboard/accessibility matrix open |
| Distribution/governance | Locked dependency license inventory and source release receipts | PARTIAL: Apache-2.0 and contributor/security/conduct/CODEOWNERS/templates implemented; pinned Fedora format/Clippy/unit/real headless/declaration checks and guarded labels added. BLOCKED desktop RSA advisory; hosted/live-label/branch-rule, broader nightly/fuzz/hardware CI and qualified package/signing/provenance remain |

Native endpoint-bound certificate review/pinning and changed/stale guards have four policy units and eight authenticated local native groups. Real Windows authentication/framebuffer decoding passes with an existing approved leaf; the Fedora SSH-attested leaf is verified and GNOME service authentication/handover/GDM rendering passes. Owner clipboard/keyring/session data are never synthetic fixtures; authorized external credential lookup is read-only. Optional browser/agent adapters and later macOS builds remain staged scope.


Authorized Fedora SSH headless/real remote-PTY acceptance passed on 2026-10-11. Mixed terminal/RDP/Files checkpoint restoration and normal application activation are implemented; detailed remaining view-management gates apply. Every remaining full-product gate above continues to apply.


Owner requires automatic GNOME account selection and sign-in on Connect. The supported English Fedora GDM path now passes on a real authorized host. Universal/unattended background login remains unqualified; assisted delivery remains available for unsupported screens.


## Native remote Codex image route

Attach PNG to remote Codex… uses an explicit registered SSH profile, absolute remote daemon socket, stable pane ID and existing private staging directory. Endpoint ID, resolved account/address and explicit confirmation bind the reviewed destination. Profile edits require a fresh review; focus/tab/local-pane/profile changes cancel ongoing delivery. Passwords and key passphrases use the existing Secret Service/libssh2 components, strict known-host verification and selected private key; unsupported gateways are refused. The shared binary PNG sender/receiver retains hash/dimensions/mode/path/correlation and remote foreground guards. Four remote workers and per-read/transfer deadlines bound work outside GTK; the local daemon worker continues serving input/rendering. Local staging reserves three copies for this route. No automatic submission or retry occurs.

Actual isolated Wayland/GDK plus generated-key authenticated loopback SSH passes native confirmation, invalid target and changed-profile refusals, real Codex draft attachment, frontend-close retention and actual image consumption. Existing local focus cancellation and ordinary-terminal refusal also pass. This is real SSH protocol/provider integration on CVF, with synthetic local input; it is not an external EVF consumption or physical desktop pass. The explicit destination is held for one operation; saved remote contexts, pane discovery and other agent adapters remain open. Remote receiver files remain available for the draft and require managed cleanup in the chosen staging directory.


## Remote Codex pane discovery

Find remote Codex panes reads all bounded job pages over the reviewed SSH profile and explicit daemon socket. On current engines, the read-only agent.probe method reuses the same-UID bounded process-group/child check used by image delivery, so a Node launcher is recognized without argv/environment/conversation inspection. It reports process presence, foreground PID and selected state only; it does not infer the model, authentication, readiness or consumption. Older engines retain exact executable-name suggestions and manual pane entry. No daemon/session is started or replaced.

The native list excludes ordinary/ended processes and displays session/window names, PID and stable pane ID. Selection resets confirmation and binds the foreground PID; delivery rejects a changed foreground process before transfer. Profile/socket/window changes cancel in-flight discovery, and stale completed rows refuse selection. Discovery and delivery share the four-worker cap; paged membership/cursor/count/identity guards are reused and the list scrolls within a bounded dialog area. Actual owned Wayland/GDK, generated-key authenticated loopback SSH and provider consumption pass the complete discovered-pane journey. External-host consumption, saved destination contexts and other agent adapters retain their gates.


## Bounded GUI metadata event feed

Authenticated subscribe/poll/unsubscribe now cover coalesced tab/focus/layout/RDP state observations, with four typed filters, monotonic per-instance cursors, actor UID binding,64-subscription and256-event bounds,60-second idle expiry and explicit overflow/resynchronization. Metadata reads do not enable focus/lifecycle/input permissions. Four actual GTK/PTY/authenticated-loopback RDP groups plus component actor/filter/expiry/capacity/gap/instance tests pass. Local controls are synthetic; no physical or external-host pass is implied. The documented feed polls100ms observations and excludes clipboard/terminal/credential/agent content. Exact-transition journaling, push streaming and remote input remain separate scope.
