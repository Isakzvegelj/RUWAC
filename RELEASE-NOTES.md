# RUWAC native PRIMARY and click acceptance

The compiled GTK frontend now has a separate test-only click-dispatch action,
so native middle-click behavior can be exercised without bypassing the click
handler. Existing low-level terminal mouse fixtures remain available.

A reproducible fixture creates its own headless Mutter display and private
D-Bus, runtime, profiles and clipboard state. It tests real Wayland PRIMARY
copy/paste, native middle-click handler paste, application SGR press/release,
Shift bypass, disabled mouse routing and separation of CLIPBOARD and the
internal Screen buffer. Generated input remains synthetic; physical input and
Windows resize/clipboard qualification remain separate acceptance gates.

Build and publication use the existing scripts and controller-managed version.
Fedora testing bundles remain distinct from qualified RPM distribution.
RUSTSEC-2023-0071 remains visible and unsuppressed.
