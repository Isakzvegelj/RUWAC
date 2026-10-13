# Fedora testing-bundle prerequisite repair

Install the OpenSSL command-line utility alongside its development libraries in
the existing hosted bundle job. Its three certificate fixtures require the utility;
the preceding job passed 104 desktop tests but failed those fixtures with a missing
executable. Verify availability before compiling. No test is skipped or downgraded.

Controller-owned source publication, exact-commit quality/privacy gates, immutable
asset upload, notices and archive verification remain in the existing workflow.
The previous published source reference is retained. Testing bundles remain unsigned
and retain the unresolved RSA advisory; external Windows and physical-input
qualification and signed RPM requirements remain open.
