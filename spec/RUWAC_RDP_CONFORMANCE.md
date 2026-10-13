# RDP monitor and codec conformance

The client advertises SUPPORT_MONITOR_LAYOUT_PDU for the existing monitor parser,
independently of the EGFX flag. Before FontMap, a decoded notification leaves
finalization waiting. After activation, it is metadata and does not reset pixels.
Bitmap activation or EGFX ResetGraphics remains authoritative for framebuffer
size. The original baseline failed all three generated wire/sequence checks.
Malformed/truncated packets still fail decoding.

ClearCodec short-bar misses decode YOn from the low byte and YOff from the next
six bits. [Microsoft Example 4](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-rdpegfx/266bbde5-a651-41a9-a5b7-721b7c4f2663)
provides an independent 70-byte bands vector; generated single-column images
verify every BGRA row through the production decoder. This is codec conformance,
not a real Windows desktop login or resize pass.

SRL complete zero chunks can end at the component boundary without a following
nonzero value. Adaptive state is retained across DWT bands. The specified trailing
zero byte remains mandatory; malformed, missing-padding and truncated inputs are
rejected. The broader padding-omission experiment is preserved privately and is
not adopted as conformant behavior.

Pinning and full file hashes cover ironrdp-pdu 0.9.0 from its registry archive,
with only the documented bands decoder modified. Original MIT/Apache texts are
retained. The headless daemon remains independent of all graphical/RDP crates.
Existing Windows connection/rendering defaults remain; EGFX interoperability and
Display Control channel negotiation still require a fresh authorized real-host
acceptance when it will not disturb an active owner session.
