# RUWAC rollback recovery fix

A deliberate rollback now disables unattended installation before switching to the
verified previous artifacts. The timer cannot immediately reinstall the reverted
update. Notification preferences and running terminal and RDP sessions are retained.
Recovery does not depend on a running user service manager. Invalid receipts, locked
preferences and preference-write failures prevent artifact changes. The GTK Updates
dialog reflects the disabled setting and permits another recovery attempt.

Validation covers portable updater faults and actual GTK callbacks on an isolated
Wayland compositor with generated helper responses. Physical desktop acceptance and
qualified Fedora binary distribution remain separate gates. See
[installation and updates](spec/RUWAC_UPDATES.md). The desktop RSA advisory remains
unsuppressed; historical privacy cleanup remains subject to owner authorization.
