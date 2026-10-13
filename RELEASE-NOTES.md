# RUWAC workflow execution and artifact privacy repair

The quality workflow now uses a literal multiline shell block for the two
publication regression suites. YAML previously folded them into one command,
which failed before source qualification. New tests parse the real workflow,
execute its exact shell body and reproduce the rejected folded form. PyYAML is
an explicit CI test dependency; it adds no headless or desktop runtime dependency.

Artifact privacy handling now decodes bounded exception-frame records and masks
only numeric PC address/range fields that resolve to executable sections. A linked
address can coincidentally contain a short identifier's bytes; that does not make
it a stored machine/account name. CIE augmentation and other data remain scanned.
Unsupported/truncated encodings and out-of-code ranges fail closed. Generated
ELF tests verify numeric coincidences, real short/long strings and malformed
records; private keys, paths, URLs and other string findings remain blocked.
No private matching value or operational receipt is published.

Mixed nesting now separates GNU Screen's documented default retained-window
policy from explicit display-size adaptation with -A. The fit-size matrix retains
all sixteen depth/prefix/input/persistence checks; a separate real default case
forces 80×24, verifies reattachment preserves it, then requires propagation of an
actual outer resize to 137 columns. Leaf PID/start time, exact bytes and termios
restoration remain required. No owner session is accessed.

The normal build/release flow reruns functional, package, privacy and certificate
checks. Application Rust and dependency source are unchanged except for versions
assigned by the controller. The measured v0.2.35 performance evidence is retained
with source equivalence checks, not reported as a new benchmark. Existing tags
and history remain immutable. Artifact publication still requires successful
exact-commit quality/privacy checks; dependency advisories remain unsuppressed.

Source qualification, testing RPM delivery and complete physical/external/security
acceptance remain distinct. This repair changes no daemon, RDP, clipboard,
credential, profile, file-transfer or workspace behavior.
