# Release Process

This project publishes self-signed Windows community releases because it does not currently have a
commercially trusted Authenticode certificate. Windows will still report an unknown publisher. A
self-signed signature protects installer integrity but does not prove the publisher's legal identity.

## How releases are signed

For every version tag, GitHub's temporary Windows runner:

1. Generates a new RSA-3072 self-signed code-signing certificate.
2. Builds the Tauri NSIS and MSI installers.
3. Signs and timestamps both installers.
4. Confirms each signature uses the generated certificate and has no hash mismatch.
5. Publishes the public `.cer`, certificate details, and SHA-256 checksums with the installers.
6. Destroys the private key when GitHub disposes of the temporary runner.

The private key is never committed or published. Each release uses a different certificate, so
trusting one release certificate does not trust later releases automatically.

## Create a release

1. Update the same version in `package.json`, `package-lock.json`, `src-tauri/Cargo.toml`, and
   `src-tauri/tauri.conf.json`.
2. Run `npm test` and `cargo test --manifest-path src-tauri/Cargo.toml`.
3. Commit and push the release preparation.
4. Create and push an annotated version tag, such as `v1.2.0`.
5. Watch the **Signed Windows Release** workflow.
6. Confirm the NSIS `.exe`, MSI `.msi`, `.cer`, `CERTIFICATE.txt`, and `SHA256SUMS.txt` appear on the
   GitHub release.
7. Compare downloaded hashes with `SHA256SUMS.txt` before installation.

Never commit a PFX, private key, certificate password, signing token, or cloud-signing credential.
