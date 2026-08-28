# Security policy

**Supported:** the latest release only.

**Report privately** via GitHub Private Vulnerability Reporting (Security tab →
Report a vulnerability). Acknowledgement within 72 hours — an honest number
for a small team, not a promise we'd break.

**In scope, absolutely:** anything that lets a non-allowlisted origin receive
input events; anything that causes an outbound network connection; pairing
token disclosure to another origin; remote code execution.

**Also in scope, and we mean it:** any claim in the README or docs that is not
true of the code. This project's product is being auditable; a false claim is
a vulnerability in that product.

**Out of scope** (see docs/THREAT-MODEL.md for reasoning): attackers already
executing code as your user, compromised browsers/extensions, physical access,
and a paired ComboForge page doing what pairing authorizes.
