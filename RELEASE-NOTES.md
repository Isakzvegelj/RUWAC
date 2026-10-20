# Explicit SSH checks and reliable update readiness

RUWAC contacts saved machines for key-access checks only after **Check now**.
Checks use public-key-only OpenSSH authentication and existing host trust,
including configured jump routes. Results identify the accepted SHA-256 key
when known. Additions start unselected and require a separate confirmation;
changed inventory, keys or trust invalidate the result. Closing the review
cancels its owned helpers. Existing credentials and chosen identities are retained.

Update controls distinguish the latest published tag, latest eligible release,
installed version and running frontend, including mixed installations. Failed
quality/privacy checks or missing package/attestation assets hold installation
with a specific reason. Downloaded hashes and provenance are verified before
installation; installed receipts, command links and binary hashes are checked
before reporting success. Publication requires artifact-bound frozen technical gates and
exact-commit hosted quality/privacy checks.

RPM directory modes are explicit, and repeated assembly varies the inherited
default ACL. Running daemons and terminals are preserved by installation;
reopen the frontend when ready. RDP reconnects after reopening are the accepted
lifetime boundary.

External Windows, physical input and provider-account qualification remain
separate from generated fixtures. The documented RSA dependency advisory and
signed Fedora repository qualification remain open.
