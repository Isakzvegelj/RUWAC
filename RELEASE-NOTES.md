# RUWAC RDP monitor and codec conformance fixes

Advertise monitor-layout support independently of the graphics pipeline. Monitor
notifications before FontMap keep activation waiting; active notifications no
longer disconnect the session or replace canonical pixels. Truncated packets
retain decode failures.

Correct ClearCodec short-bar coordinate bits against Microsoft's Example 4 and
SRL decoding when complete zero chunks end a component. Required SRL padding and
stream bounds remain enforced. Pinned PDU source, dependency notices and modified
file checksums accompany the codec patch.

Generated conformance packets and isolated authenticated RDP/native regressions
remain distinct from real Windows acceptance. The Windows graphics experiment is
preserved separately; the existing published connection defaults are retained.
No new login to an owner's active Windows session is used as evidence. Windows
dynamic display, physical input, RSA remediation and signed RPM qualification
remain open. Source and testing bundles do not imply whole-product qualification.
