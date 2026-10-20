# Deterministic Fedora RPM assembly

Fedora packages now record the source commit timestamp instead of the packaging
machine's current time. The release workflow repeats assembly in a separate
directory and requires identical complete core and desktop RPM bytes before
publishing them.

This verifies packaging from fixed compiled inputs. Independent reproducible
compilation, signed Fedora repository delivery and the documented desktop RSA
dependency finding remain separate open qualifications.
