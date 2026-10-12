# GNOME connection compatibility

Pinned IronRDP connector 0.10.0, MIT OR Apache-2.0. Archive identity and shipped file hashes are recorded in UPSTREAM.json. Existing CredSSP and cryptographic implementations are preserved.

RUWAC answers connect-time Bandwidth Measure Stop with measured elapsed time and byte count, following MS-RDPBCGR 2.2.14.2.2. A Stop without Start is rejected; measurement state resets per connection. Continuous measurement remains the active-session layer responsibility.

Optional configuration advertises the Graphics Pipeline only with 32-bit GNOME graphics enabled. Standard/Windows RDP retains its original negotiation. For an authenticated GNOME same-endpoint enhanced redirection, a separate opt-in branch negotiates RDSTLS and the redirected session ID. RUWAC completes the bounded RDSTLS exchange under verified TLS before resuming the existing connector. It forwards the server-issued opaque encrypted handover credential; the original service password and separately saved Linux password are not substituted. Original endpoint trust and the authenticated leaf must both match. Arbitrary redirected destinations are refused.

Negotiation routing tokens and Client Info are omitted from debug diagnostics. This patch does not remediate the RSA advisory or implement unattended GDM authentication. GNOME code was inspected for protocol behavior; no GPL source was copied. Replace these adapters with upstream components when equivalent acceptance evidence exists.
