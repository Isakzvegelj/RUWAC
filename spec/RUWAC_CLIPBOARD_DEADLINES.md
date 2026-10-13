# Desktop clipboard cancellation and deadlines

Every desktop text, PNG staging and RDP clipboard read has a five-second deadline
covering selection negotiation and byte streaming. Focus or clipboard ownership
changes invalidate the existing FocusBroker ticket and cancel a blocked read without
waiting for another byte. Cancelled PNG reads remove only their owned partial file;
no cancelled payload is pasted, staged or promoted. Normal payload caps and separate
Screen/internal buffers remain unchanged.

The guard polls cancellation on the GTK main context every 25 ms. Selecting out a
GIO future cancels its own pending operation; operation completion removes its timer.
Checks before and after reads and stream close reject a late owner change. PNG decode
continues on the existing bounded worker and checks the ticket before publication.

Validation covers 107 desktop units, including a real kernel socket stream stalled
until deadline or focus cancellation, followed by a successful fresh read on the same
stream. Six actual GDK cases use a deliberately incomplete same-process provider on
an owned Wayland compositor. The baseline text and PNG focus cases remained pending
after seven seconds; the fixed cases cancelled around 200 ms after start when focus
changed at 200 ms. Unchanged focus stopped at five seconds. All fixed cases removed
partial staging files and kept main-context timer ticks running.

Reproduce with an explicitly isolated display and private evidence directory:

```sh
cargo build --locked --manifest-path ruwac/desktop/Cargo.toml --example clipboard-deadline-fixture
python3 ruwac/tests/integration/clipboard_deadlines.py --driver "$BUILD_ROOT/target/debug/examples/clipboard-deadline-fixture" --evidence "$PRIVATE_EVIDENCE" --owned-display
```

The driver refuses ordinary display names and requires an owned private runtime
directory. This is actual GDK/GIO integration with generated data. It does not qualify
cross-process clipboard managers, external hosts or physical input.
