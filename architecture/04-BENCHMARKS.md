# Benchmark evidence

Current desktop interaction and embedded IronRDP state is recorded in [the acceptance report](../spec/RUWAC_ACCEPTANCE_STATUS.md) and [current architecture decisions](../spec/RUWAC_ARCHITECTURE_ADR.md). Earlier measurements and design limits below are historical evidence.


CVF, Fedora Rust 1.98.1, tmux 3.7c, kernel 7.2.9-200.fc44.x86_64. Historical pre-rename optimized binary SHA-256 `b117dc99981695a658e2b1c7a2b02774ed70eba5e2b42f92aa5cb94d98f886b3`. Six virtual CPUs. Before/after machine CPU busy fractions were 0.75%/0.79%; both meet the declared <20% idle qualification. These snapshots do not eliminate all VM scheduling noise. No other owned build ran during measurements.

30 samples per operation/size, three warmups, alternating product order. Both use real subprocess CLI RTT against isolated Unix sockets and identical non-executing Python raw receivers. Pane counts are windows with one pane each. Send acknowledges queue admission. Capture is bounded canonical text; tmux has different text/format semantics. This measures process/API latency, not physical display latency.

| Panes | Operation | ruwac median/p95 ms | tmux median/p95 ms | Median ratio |
| --- | --- | --- | --- | --- |
| 1 | list | 1.254/1.400 | 2.279/2.438 | 0.55× |
| 1 | send | 1.283/1.462 | 2.250/2.463 | 0.57× |
| 1 | capture | 1.283/1.464 | 2.226/2.413 | 0.58× |
| 5 | list | 1.329/1.453 | 2.300/2.585 | 0.58× |
| 5 | send | 1.325/1.421 | 2.217/2.415 | 0.60× |
| 5 | capture | 1.289/1.381 | 2.206/2.392 | 0.58× |
| 20 | list | 1.409/1.506 | 2.398/2.611 | 0.59× |
| 20 | send | 1.299/1.433 | 2.285/2.461 | 0.57× |
| 20 | capture | 1.348/1.518 | 2.299/2.423 | 0.59× |
| 40 | list | 1.512/1.624 | 2.495/2.662 | 0.61× |
| 40 | send | 1.294/1.384 | 2.330/2.500 | 0.56× |
| 40 | capture | 1.353/1.448 | 2.301/2.384 | 0.59× |

**Gate result: all 12 API medians are within 2× tmux.** This is evidence for this slice/fixture, not a general speed ranking.

Separate key-to-render: median 35.47 ms, p95 42.12 ms, n=30. A key marker goes through the dedicated TUI PTY and child; the observer detects its hex echo in emitted canonical render (5 ms observer granularity). No photon or human visual claim.

At 40 panes, daemon RSS 13.83 MiB and TUI RSS 5.07 MiB. A one-second idle sample measured 0.0%/0.0% of one CPU core; child interpreters excluded. Short CPU samples are coarse.

Separate warm-cache startup (n=30):
- ruwac: median 5.70 ms, p95 6.35 ms.
- tmux: median 15.74 ms, p95 26.41 ms.
- ruwac client spawn to first canonical grid: median 5.66 ms, p95 6.35 ms.

Startup definitions differ: ruwac starts an empty daemon and queries health; tmux creates a cat child and then lists sessions. Do not use these as equivalent cold-start rankings.

Raw samples: [benchmark-results.json](benchmark-results.json). Reproduce only under the designated build workspace:

```sh
cd ruwac
PYTHONDONTWRITEBYTECODE=1 python3 tests/benchmarks/api.py --binary "$CARGO_TARGET_DIR/release/ruwac" --evidence "$EVIDENCE_DIR"
PYTHONDONTWRITEBYTECODE=1 python3 tests/benchmarks/startup.py --binary "$CARGO_TARGET_DIR/release/ruwac" --evidence "$EVIDENCE_DIR"
```

Earlier ttmux numbers used five samples on a busier host and are not combined with this new run. No fresh ttmux comparison was performed; tmux is the required measured baseline. High-volume throughput, long soak and physical touchpad/desktop cases remain open.

Startup samples used the immediately preceding binary before the shutdown-without-reader correction; its exact hash is retained separately in the raw receipt. API and render figures above were refreshed for the reviewed binary.
