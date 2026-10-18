# RUWAC persistent SSH and agent launch context

New saved SSH and agent launches retain explicit endpoint, pane/process and
provider associations across frontend replacement and independent windows.
Restore reads private metadata without starting a connection or an agent.
Agents uses recorded provider kind instead of matching words in session names,
and selects the recorded pane/window after checking current daemon membership.

Image paste retains the working local/remote workflow and the previous bounded
upload/focus guards. It is now scoped to the original SSH pane and process; a
new local split in the same session does not acquire its destination. Changed
or deleted endpoint identities refuse delivery rather than selecting another
host. Credentials remain in Secret Service, and metadata does not duplicate
credential references or generated SSH command lines.

Explicit reconnect reconstructs the launch recipe using the installed client.
Private atomic catalog records are daemon-bound, bounded and checked for unsafe
file types/permissions. Storage failures are shown while volatile context and
the live connection remain usable. Older sessions without metadata are not
retroactively associated by guessing their names or terminal output.

Qualification distinguishes actual loopback SSH and native GDK/GTK with synthetic
input from real external-provider and physical-desktop acceptance. Seamless RDP
frontend handoff, heuristic PTY drain ordering, broad external interoperability,
full distribution qualification and the unsuppressed RSA advisory remain open.
Existing build/release scripts and their controller own publication/versioning.
