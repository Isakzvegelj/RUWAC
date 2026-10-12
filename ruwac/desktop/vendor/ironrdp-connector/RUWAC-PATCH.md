# Connect-time bandwidth measurement compatibility

Pinned IronRDP connector 0.10.0, MIT OR Apache-2.0. Unchanged authentication, TLS, licensing and cryptographic implementations. The Cargo manifest is the registry-normalized manifest; original crate identity and archive hash are in UPSTREAM.json. Apache license text is retained.

RUWAC answers connect-time Bandwidth Measure Stop with the measured elapsed time and payload byte count, as required by MS-RDPBCGR 2.2.14.2.2. GNOME waits for this reply before proceeding to licensing; ignoring it stalls Remote Login. Measurement state is per connector and resets on every Start/Stop. A Stop without Start is rejected. Continuous measurement is left to the active-session layer. RTT replies remain unchanged.

This compatibility patch does not remediate the upstream RSA advisory and does not implement unattended GDM authentication. GNOME source was inspected for protocol behavior; no GPL source was copied. Keep the patch until an upstream fixed connector can replace it with identical acceptance evidence.
