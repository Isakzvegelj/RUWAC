# RUWAC initial Dynamic RDP sizing

New Dynamic RDP connections wait for a stable allocated viewport before the existing credential lookup and authenticated connection start. This prevents a newly opened tab from negotiating a temporary 1280×720 desktop and immediately resizing again. The wait is bounded; Disconnect or changed settings cancel the queued request. Fixed/local-scaled display modes retain their behavior.

A new background tab uses the allocated workspace without selecting itself or taking clipboard ownership. An existing hidden connection retains its last actual framebuffer size. Existing daemon, terminal sessions, profiles, clipboard and authentication infrastructure are preserved.

Qualification uses actual GTK/Wayland, disposable Secret Service credentials and an authenticated loopback peer. Three initial window sizes, queued cancellation, background connection, reconnect, clipboard and neighboring transports are tested separately from physical input and external Windows/Fedora interoperability. The RSA advisory and complete signed Fedora distribution remain open; this source release does not claim full product acceptance.
