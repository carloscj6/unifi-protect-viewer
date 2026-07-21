# Release Process

Official Windows downloads must be Authenticode-signed with a publicly trusted code-signing
certificate. Do not publish a self-signed or unsigned installer as an official release.

## One-time GitHub setup

Obtain a code-signing certificate exported as a password-protected `.pfx` containing its private
key. In the repository's **Settings > Secrets and variables > Actions**, create:

- `WINDOWS_CERTIFICATE`: Base64 text containing the complete PFX file.
- `WINDOWS_CERTIFICATE_PASSWORD`: The PFX export password.

Create the Base64 value without printing it into logs:

```powershell
[Convert]::ToBase64String([IO.File]::ReadAllBytes('C:\secure\certificate.pfx')) |
  Set-Clipboard
```

Paste the clipboard content directly into the GitHub secret. Keep the original PFX outside the
repository and never commit it.

## Create a release

1. Update the same version in `package.json`, `package-lock.json`, `src-tauri/Cargo.toml`, and
   `src-tauri/tauri.conf.json`.
2. Run `npm test` and `cargo test --manifest-path src-tauri/Cargo.toml`.
3. Commit and push the release preparation.
4. Create and push an annotated version tag, such as `v1.2.0`.
5. Watch the **Signed Windows Release** workflow.
6. Confirm both the NSIS `.exe` and MSI `.msi` appear on the GitHub release.
7. Download an installer and confirm Windows reports a valid digital-signature publisher.
8. Compare the installer hash with `SHA256SUMS.txt`.

The workflow refuses to publish when the certificate is missing, expired, lacks a private key, or
when any generated installer fails Windows Authenticode verification.
