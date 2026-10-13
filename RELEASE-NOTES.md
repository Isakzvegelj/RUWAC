# RUWAC publication resilience and measured RDP latency

The product-owned quality wait retries bounded GitHub timeout/429/5xx failures
without treating them as passing checks. Access failures and completed source or
privacy failures stop publication. Exact commit/workflow selection is preserved;
dependency advisories remain separate and unsuppressed. Five transport/process
fault tests cover retry, recovery, deadlines, access denial and immutable commit
selection. Shared build/release infrastructure and version allocation are unchanged.

Authenticated same-endpoint FreeRDP comparisons now measure actual timestamped
X11 pixels, first-frame time, CPU and RSS. Ten alternating trials per client with
fifty frames each establish an A/B/A baseline. Disabling TCP packet coalescing for
interactive RDP reduced measured TCP-accept-to-first-frame time from about 203 ms
to 121 ms in the software fixture. TLS/CredSSP and explicit certificate trust
remain intact. GTK paint latency and memory remain higher than the FreeRDP CLI.
See the benchmark report for exact scopes, variation and empirical regression budgets.

This qualification uses generated loopback desktops and owned compositors.
Windows/physical/3K interoperability, signed RPM repository, unattended RPM
installation and desktop RSA remediation remain separate open requirements.
Operational source releases and testing RPMs do not prove whole-product security.
