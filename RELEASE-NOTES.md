# RUWAC natural terminal grids and status controls

Current engines advertise up to 512 rows, 1024 columns and a 262144-cell budget. GTK and TUI clients use the actual allocation within those bounds. Smaller simultaneous attachments retain the documented minimum-size policy. Existing running daemons keep their capabilities and live processes; old engines retain their previous 100-row/200-column limits until an owner-controlled update.

Requests and ordinary replies remain limited to 1 MiB. Terminal render clients explicitly opt into bounded replies up to 64 MiB. Oversized responses receive a correlated error before any partial frame is written, keeping the connection usable. Large snapshots use a separate bounded server budget. Checkpoints with larger active or retained buffers use schema 6; existing smaller archives remain compatible, and older readers preserve files they cannot read.

The CLI status bar renders and hit-tests one shared layout: clipped Unicode labels stay clickable, the selected window stays visible, and arrows navigate hidden windows. Bottom-row clicks remain local. Native rendering also avoids redundant frame copies and unnecessary blank-glyph layout while retaining styles and decorations.

Qualification distinguishes actual private PTYs, kernel sizes, generated colour frames, checkpoint files and native GTK/Wayland pixels from physical input and external-host interoperability. The RSA advisory, full Fedora distribution qualification and seamless RDP frontend handoff remain separate open gates.
