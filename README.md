# Unifi Protect Viewer

A Windows desktop app that automatically opens a UniFi Protect camera wall and keeps it visible on
an unattended store or office monitor. It is built with Tauri 2 and does not use Electron.

> **Best use:** Install this app on a dedicated camera-viewer PC that is rarely used for other work.
> The app deliberately returns to the foreground and fullscreen after inactivity. That is useful for
> a camera display, but disruptive on an employee's everyday workstation.

## What the app does

- Starts with Windows and opens the selected camera profile automatically.
- Saves multiple store, console, or camera-wall profiles.
- Tests the UniFi address before saving it.
- Opens the configured UniFi Protect dashboard or live-view URL.
- Reuses WebView2's encrypted UniFi login cookies between launches.
- Automatically enters the saved username and password when UniFi asks for them again.
- Leaves MFA visible so a person can enter the one-time code when UniFi requires it.
- Enters both Windows-app fullscreen and UniFi's internal camera fullscreen.
- Hides UniFi navigation and non-camera dashboard controls when possible.
- Restores the camera viewer to the foreground after 60 seconds without keyboard or mouse input.
- Keeps running in the background when someone clicks the window's `X` button.
- Starts only one copy of the viewer at a time.
- Encrypts saved passwords with Windows DPAPI.
- Provides connection diagnostics, logs, recovery actions, and support reports.

## Before installing

You need:

1. A Windows 10 or Windows 11 PC.
2. Microsoft Edge WebView2, which is normally already installed on current Windows systems.
3. Internet or local-network access to the UniFi console.
4. The complete URL of the camera view you want displayed.
5. A dedicated UniFi viewer account and password.
6. The ability to complete UniFi MFA during initial setup, if enabled.

For the safest setup, give the viewer account only the permissions needed to view cameras. Do not
use an owner's or administrator's everyday account unless there is no alternative.

## Installation for nontechnical users

1. Obtain the `.exe` installer from your technician or release package.
2. Double-click the installer.
3. If Windows asks for permission, select **Yes**.
4. Follow the installer prompts.
5. Open **Unifi Protect Viewer** from the Start menu.

The app may immediately occupy the whole screen. Press `Esc` if you need to leave fullscreen while
configuring it.

## First-time setup

### 1. Create a camera profile

1. Open the app's configuration screen.
2. Enter a descriptive **Profile Name**, such as `Front Store Cameras`.
3. Paste the complete **UniFi Protect URL**.
4. Enter the dedicated UniFi viewer username.
5. Enter the account password.
6. Leave the Protect version set to automatic unless a technician instructs otherwise.
7. Select **Test Connection**.
8. Wait for a successful result.
9. Select **Save and Launch**.

Do not enter only `unifi.ui.com`. Open the exact camera dashboard or live view in a normal browser
first, then copy its complete address. Typical addresses contain `/protect/dashboard` or
`/protect/liveview`.

### 2. Complete the first UniFi login

The app enters the saved username and password automatically. UniFi may then display an MFA page.

1. Obtain the one-time code from the account's authenticator, email, or approved MFA method.
2. Enter the code directly in the viewer.
3. Approve or remember the device if UniFi offers that option.
4. Wait for the camera dashboard to open.

The app preserves UniFi's encrypted WebView2 session. Future launches normally go directly to the
cameras. When UniFi expires the session, the app submits the saved login again. A person must still
complete MFA whenever UniFi requires it.

### 3. Configure automatic startup

In the **Startup** section:

1. Select the profile that should open automatically.
2. Select the monitor used for cameras.
3. Enable fullscreen startup.
4. Enable start with Windows.
5. Enable automatic reconnect.
6. Save the startup settings.
7. Restart the PC once and confirm the camera wall returns without assistance.

## Everyday operation

Under normal operation, no one needs to touch the app. Start the PC, sign into Windows if required,
and allow the viewer to open the saved camera wall.

### Fullscreen controls

| Key | Action |
| --- | --- |
| `F` | Enter Windows-app fullscreen and UniFi camera fullscreen |
| `Esc` | Exit both fullscreen modes |
| `F11` | Toggle Windows-app fullscreen |
| `F9` | Restart the viewer |
| `F10` | Select another saved profile |
| `Ctrl+Shift+F10` | Open technician configuration and support tools |

The letter shortcuts are ignored while typing in a text field.

### What happens after inactivity

The app checks Windows' system-wide keyboard and mouse idle time in the native background process.
After 60 seconds without input, it:

1. Shows the viewer if it was hidden.
2. Restores it if it was minimized.
3. Brings it to the foreground.
4. Makes the Windows app fullscreen.
5. Activates UniFi's internal camera fullscreen.

This behavior also works when the viewer is behind another program because it does not depend on a
background webpage timer.

### Closing and stopping the app

Clicking the window's `X` does **not** stop the viewer. It hides the window and leaves the native
background watchdog running. The viewer returns after 60 seconds of system inactivity.

To stop it completely for maintenance:

1. Open Windows Task Manager with `Ctrl+Shift+Esc`.
2. Find **Unifi Protect Viewer**.
3. Select **End task**.

If start with Windows is enabled, the viewer starts again after the next Windows sign-in. Disable
automatic startup in the app before maintenance if it should remain off.

## Multiple locations or camera walls

Create one profile for each store, console, or view. Press `F10` to select a different profile. The
startup profile is the one opened automatically after launch or reboot.

Each profile stores its own URL, username, encrypted password, and Protect-version fallback. Use
clear names so an onsite user can select the correct location without recognizing the URL.

## Troubleshooting for onsite users

### The screen is black or keeps loading

1. Wait one minute for automatic recovery.
2. Press `F9` to restart the viewer.
3. Confirm the PC has internet or access to the camera network.
4. Confirm the UniFi console is powered on.
5. Press `Ctrl+Shift+F10` and run **Test Connection**.
6. If it still fails, create a support report and send it to the technician.

### The UniFi login page is visible

- Wait briefly; the app should enter the saved username and password.
- If MFA appears, enter the current one-time code.
- If UniFi reports an incorrect password, open configuration with `Ctrl+Shift+F10`, update the
  password, test the connection, and save again.

### Cameras appear, but they are not fullscreen

1. Move the mouse over the camera view once.
2. Press `F`.
3. Stop using the keyboard and mouse for 60 seconds; the native watchdog should restore fullscreen.
4. If it still fails, press `F9`.

### The viewer keeps covering another program

This is expected on a dedicated viewer PC. The app reclaims the foreground after one minute of
inactivity. Completely stop it through Task Manager or disable its Windows startup setting before
using that PC for other work.

### The app was closed but returned

Clicking `X` only hides it. This is intentional. See **Closing and stopping the app** above.

## Technician deployment checklist

1. Use a dedicated Windows account with automatic sign-in only if company policy permits it.
2. Prevent sleep, hibernation, and monitor power-off where continuous viewing is required.
3. Set Windows display scaling and resolution before selecting the viewer monitor.
4. Create a least-privilege UniFi viewer account.
5. Save and test the exact Protect dashboard/live-view URL.
6. Complete MFA and approve the device.
7. Enable Windows startup, fullscreen, and reconnect.
8. Reboot and confirm unattended recovery.
9. Test `Esc`, `F`, `F9`, and `Ctrl+Shift+F10`.
10. Click `X`, wait 60 seconds without input, and confirm the viewer returns to the foreground.
11. Disconnect and reconnect the network to test recovery.
12. Record how onsite staff can reach support.

For a rollout-specific checklist, see [Store Camera PC Deployment](docs/STORE_DEPLOYMENT.md).

## Privacy and security

- Saved passwords are encrypted with Windows DPAPI before being written to disk.
- UniFi cookies remain in WebView2's app-specific encrypted data directory.
- Passwords are excluded from support reports.
- Navigation is limited to local viewer pages and configured UniFi origins.
- Camera pages use explicit Tauri commands rather than a generic compatibility bridge.
- There is no Electron runtime or Electron IPC layer.

Anyone who can use the configured Windows account may still be able to open the camera viewer. Lock
down the dedicated PC according to the organization's physical-security requirements.

## Building installers

Installers are generated in:

- NSIS: `src-tauri/target/release/bundle/nsis/`
- MSI: `src-tauri/target/release/bundle/msi/`

Development prerequisites are Node.js 20+, Rust, and the current Tauri Windows prerequisites.

```powershell
npm install
npm start
```

Run validation and create optimized installers:

```powershell
npm test
cd src-tauri
cargo test
cd ..
npm run build
```

## Project structure

```text
src-tauri/src/lib.rs    Native commands, secure storage, window lifecycle, and idle watchdog
src/html/               Setup, profile selection, recovery, and support pages
src/js/preload.js       Camera-page login, fullscreen, and live-view automation
src/js/liveview/        Readable reference copies of automation sections
test/js/                Frontend and native integration contract tests
```

## License

[MIT](LICENSE)
