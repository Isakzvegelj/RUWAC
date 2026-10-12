# Pinned terminal library source

This is the Apache-2.0 library source from alacritty_terminal 0.26.0. Original authors and license notices are preserved. UPSTREAM.json records the registry archive and each original library-file checksum.

RUWAC adds one read-only Term::primary_grid accessor. It returns the canonical existing primary grid without swapping or mutating buffers, and does not replace parsing, resize, input, history or rendering behavior. This avoids copying a second live terminal model or disturbing the alternate screen during archival.

The normalized manifest disables upstream unit/reference targets in this source subset; their approximately 47 MB of fixtures remain in the pinned evaluation archive. RUWAC's own behavior, recovery and bounds tests exercise the adapter. An upstream refresh must review and reapply this accessor and verify all checksums; this is a maintained local patch, not a new upstream release or a security remediation.
