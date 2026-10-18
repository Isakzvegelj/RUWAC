# RUWAC terminal output and exit reliability

A terminal now reports its own process exit after draining ready final output.
One reader publishes both data and the single end event, preventing background
processes that retain the terminal from changing ended history afterward.
Continuous inherited output cannot keep the pane alive indefinitely: the final
drain is capped at 8 MiB. Closing standard streams alone does not end a running
program, and output resumes if it reopens its terminal.

Owned PTY and daemon regressions exercise delayed output delivery, final bursts,
closed/reopened streams, background writers, exit codes, live sibling windows,
ended-history reattachment and the full 128-pane capacity with a 1,024-descriptor
limit. These are automated Linux checks; broader physical/external acceptance
and the existing distribution/security gates remain separate.

The change applies to newly started daemons. Existing daemons and their live
programs retain their running implementation until deliberately replaced; this
update does not restart or migrate those processes.
