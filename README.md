# Unifi Protect Viewer

A native Tauri 2 viewer for unattended UniFi Protect camera walls on Windows store PCs.

> **Recommended use:** Install this on a dedicated viewer PC that is rarely used for other work or
> manually rearranged. The app is intentionally designed to reclaim the foreground and restore the
> camera wall after inactivity, which is useful for unattended displays but intrusive on a normal
> employee workstation.

## What it does

- Starts with Windows and opens the configured camera wall automatically
- Runs fullscreen on the selected display
- Signs into UniFi Protect and removes non-camera interface elements
- Reuses WebView2's encrypted UniFi session cookies and signs in again when the session expires
- Enters UniFi's own live-view fullscreen mode in addition to native window fullscreen
- Returns to the foreground and restores fullscreen after 60 seconds of system inactivity
- Keeps running in the background when the window is closed with `X`
- Supports multiple store or camera-wall profiles
- Retries failed loads and restarts an unresponsive renderer
- Encrypts saved passwords with Windows DPAPI
- Provides native connection testing, diagnostics, logs, and support reports
- Prevents duplicate viewer instances

## Install

Build or download either Windows installer:

- NSIS: `src-tauri/target/release/bundle/nsis/`
- MSI: `src-tauri/target/release/bundle/msi/`

For an onsite rollout, follow [Store Camera PC Deployment](docs/STORE_DEPLOYMENT.md).

## First setup

1. Open the viewer.
2. Enter a profile name and the complete UniFi Protect live-view URL.
3. Enter the dedicated viewer account credentials.
4. Select **Test Connection**.
5. Save the profile.
6. In **Startup**, select the profile, display, fullscreen, Windows startup, and reconnect options.
7. Restart once and complete the deployment acceptance test.

UniFi may require MFA the first time the viewer signs in. Complete that prompt on the viewer PC;
the trusted WebView2 session is then reused until UniFi invalidates it.

Closing the window with `X` hides it instead of terminating the viewer. After 60 seconds without
system-wide keyboard or mouse input, the native background watchdog shows the window, brings it to
the foreground, and restores both native and UniFi fullscreen. To stop it completely, end the
process or uninstall/disable it for that PC.

The connection test is a native Rust command. Renderer pages call explicit Tauri commands through
`window.__TAURI__.core.invoke`; there is no compatibility API or generic frontend channel.

## Keyboard controls

| Shortcut           | Action                                      |
| ------------------ | ------------------------------------------- |
| `F9`               | Restart viewer                              |
| `F10`              | Open profile selection                      |
| `Ctrl+Shift+F10`   | Open technician configuration and support   |
| `F11`              | Toggle fullscreen                           |
| `F`                | Enter native and UniFi camera fullscreen    |
| `Esc`              | Exit native and UniFi camera fullscreen     |

## Development

Prerequisites are Node.js 20+, Rust, and the current Tauri Windows prerequisites.

```powershell
npm install
npm start
```

Run validation and build optimized installers:

```powershell
npm test
cd src-tauri
cargo test
cd ..
npm run build
```

## Architecture

```text
src-tauri/src/lib.rs    Native commands, storage, security, window lifecycle, watchdog
src/html/               Setup, profile selection, recovery, and support pages
src/js/preload.js       Camera-page automation injected by Tauri
src/js/liveview/        Readable reference copies of automation sections
```

The native backend restricts navigation to local viewer pages and configured UniFi origins.
Sensitive commands are unavailable from camera pages. Passwords are encrypted before storage and
excluded from generated support reports.

## License

[MIT](LICENSE)
