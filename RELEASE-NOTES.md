# RUWAC native daily interaction and embedded RDP

This source prerelease adds Screen window/pane navigation, native chooser/command/help, interactive keyboard selection/search, internal paste, persistent nested horizontal/vertical splits, ratio resize and zoom. Scrollback defaults to 50,000 lines with a memory budget; paged history and copy view are bounded.

The same GTK window now includes original IronRDP integration with verified TLS/NLA, Secret Service references, native framebuffer, scaling and explicit send-key actions. Local negotiation/TLS validation and synthetic embedded rendering pass; authenticated remote-host acceptance remains blocked by missing designated endpoint and credentials. CLIPRDR is not implemented.

Core regressions, startup races, native Vim/less/htop callbacks, real cross-process GDK clipboard transport and PNG cleanup pass in owned fixtures. Physical GNOME/PRIMARY/touchpad and abrupt-child termios recovery remain unqualified. The optional desktop workspace keeps graphical/RDP dependencies out of the daemon and CLI. See the current acceptance and integration reports for measured limits.
