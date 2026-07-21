# Security Policy

## Official downloads

Use only the GitHub Releases page for this repository. Each community release includes:

- A self-signed Authenticode NSIS installer and MSI package.
- The release-specific public `.cer` certificate.
- `CERTIFICATE.txt` containing the expected certificate thumbprint.
- `SHA256SUMS.txt` containing installer hashes.

The signature detects post-signing modification but, because it is self-signed, does not establish a
publicly trusted publisher identity. Windows will normally display an unknown-publisher warning.

## Private keys

Signing private keys and PFX files must never be committed. GitHub creates an ephemeral signing key
for each release and destroys it with the temporary runner. Anyone finding a private key in an issue,
pull request, fork, artifact, or commit history should report it immediately.

## Reporting a vulnerability

Do not include passwords, session cookies, private keys, camera addresses, or support bundles in a
public issue. Contact the repository owner privately through their GitHub profile before publishing
sensitive details. Include the affected version and a minimal reproduction with secrets removed.

## Scope and responsibility

This project is a camera viewer, not a life-safety or guaranteed security-monitoring service. Users
are responsible for account permissions, network controls, physical access, updates, incident
response, and compliance with applicable laws. Modified or third-party-distributed builds are not
controlled by this repository's maintainers.
