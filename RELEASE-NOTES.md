# RUWAC bounded desktop clipboard reads

Terminal text paste, PNG staging and RDP desktop clipboard reads share a five-second
read deadline and focus/ownership cancellation. A stalled provider cannot leave the
operation pending indefinitely or retain an incomplete staging file. Per-operation
GIO reads are cancelled rather than waiting for the provider to deliver another
chunk. Completion rechecks ownership; timers are removed when operations finish.

Actual GDK tests on an isolated Wayland compositor reproduce indefinitely pending
baseline text/image reads. Six generated-provider cases verify deadline and focus
cancellation for all readers, partial-file cleanup and a responsive main loop.
Kernel-backed GIO tests verify that cancellation releases the pending operation
and permits a fresh read. These tests use a same-process stalled provider; they
are distinct from physical input and external RDP interoperability.

Normal terminal/PNG and authenticated RDP clipboard regression tests remain release
gates. Windows display/clipboard, physical input, RSA remediation and signed RPM
qualification remain open. This is a source release with a Fedora testing bundle,
not whole-product or security-qualified binary distribution.
