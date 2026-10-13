# RDP comparison and transport latency

The reproducible benchmark compares RUWAC with Fedora FreeRDP 3.32.1 against
one authenticated loopback IronRDP peer. Both clients receive the same 640×480
bitmap desktop, use native scale and verify the generated TLS certificate.
Each batch alternates ten fresh processes per client and observes fifty frames
per client. The Fedora 44 container is limited to one CPU and 1 GiB; network is
disconnected. Xvfb and Cairo provide software X11 rendering. Filesystem/font
caches are shared; these are fresh-process tests, not cold-cache tests.

Frame markers carry a sequence and a monotonic generation timestamp. The reader
matches that sequence to actual X11 pixels. Generation-to-readback includes
server construction/encoding, transport, decode, rendering and observer scheduling.
TCP acceptance is measured separately from client/UI launch. Neither metric is
physical scanout latency. The headless daemon starts before measurement; its RSS
is separate from GUI RSS. Raw fixture receipts remain private.

The released baseline and candidate were tested in A/B/A order. The last baseline
batch controls for drift after the candidate batch. These are measured medians,
not security or whole-product acceptance claims:

| Metric | Baseline reversal | Candidate TCP_NODELAY | FreeRDP in candidate batch |
| --- | ---: | ---: | ---: |
| TCP accept to first visible frame | 202.65 ms | 121.17 ms | 110.80 ms |
| Client/UI launch to first visible frame | 552.03 ms | 455.62 ms | 207.79 ms |
| Frame generation to X11 readback | 24.97 ms | 21.55 ms | 8.16 ms |
| Client RSS | 135.05 MiB | 134.33 MiB | 52.21 MiB |
| Client CPU as percentage of one core | 10.49% | 9.28% | 7.28% |

Candidate p95 generation-to-readback was 64.97 ms; p95 TCP-accept-to-visible-frame
was 222.62 ms. The isolated headless daemon used about 11 MiB. The socket change
addresses connection latency, not the remaining GTK startup, memory and rendering
gaps. Certificate inspection also uses TCP_NODELAY; eight actual Wayland certificate
regressions preserve rejection, explicit one-time/saved trust, restart behavior,
changed-certificate confirmation and stale endpoint refusal.

Empirical fixture regression budgets are: median TCP-accept-to-visible-frame at
most 250 ms, p95 generation-to-readback at most 150 ms, GUI RSS p95 at most 256 MiB,
and median launch-to-visible-frame at most 1200 ms. These bounds include measured
variation and are meaningful only with the documented one-CPU software fixture.
They do not define a WAN, Windows, GNOME, 3K, GPU or physical-desktop promise.
Those acceptance scopes remain separate. Steady-state paint latency and GUI memory
still exceed the FreeRDP CLI baseline; no claim of overall superiority is made.

Build the applications and optional fixture with locked dependencies in a separate
build directory. Install the benchmark-only FreeRDP, Xvfb, python-xlib, Pillow and
OpenSSL tools in a disposable Fedora environment. Run:

```sh
python3 ruwac/tests/benchmarks/rdp_clients.py --run-owned-fixture \
  --binary "$CARGO_TARGET_DIR/release/ruwac" \
  --gui "$CARGO_TARGET_DIR/release/ruwac-desktop" \
  --server "$CARGO_TARGET_DIR/release/examples/rdp-fixture" \
  --evidence "$PRIVATE_BENCHMARK_EVIDENCE" --trials 10 --frames 5
```

The helper creates its own display, daemon, generated credentials, certificate and
loopback endpoint. It leaves the owner's desktop, profiles and sessions alone.
Markers are enabled only for the benchmark child. Normal fixture mode retains its
original desktop/resize behavior. No remote password is accepted in an argument.
The tests stop their own processes on success and failure.

[TCP_NODELAY semantics](https://docs.rs/tokio/1.53.1/tokio/net/struct.TcpStream.html#method.set_nodelay)
are provided by the upstream Tokio socket API. The implementation preserves TLS,
CredSSP, authenticated control and clipboard ownership policy.
