# RUWAC terminal usability and connection controls

GTK terminals pass Ctrl+A directly to applications; Ctrl+Alt+A opens local RUWAC commands. New windows and splits explicitly use the login shell even with an older running daemon. Existing shells retain their original startup flags.

Live clients close when the last shell exits; ended history remains available. CLI window labels accept mouse clicks, and lowercase s / vertical-bar split commands are supported. GTK uses its full allocated terminal height and up to 200 columns; unmapped tabs wait before attaching.

RDP reconnects retain decoded desktop dimensions. Right-click a tab for connection/display controls. Concise tab titles, status icons and a centered reconnect action clarify connection state. These changes require native and authenticated loopback acceptance; external server window placement remains a separate gate.

The native About / Support dialog includes version/license information and a QR link to the dedicated RUWAC support page. No payment secrets are included.

Installing updates leaves the running process unchanged. GUI restart still interrupts RDP transports; persistent terminals survive. Seamless RDP process handoff is not implemented. Existing dependency advisories and physical-input/external-host acceptance limitations remain unsuppressed.
