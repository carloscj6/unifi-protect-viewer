# Store Camera PC Deployment

This guide is for a Windows PC dedicated to showing a UniFi Protect camera wall.

## Expected result

After Windows signs in, Unifi Protect Viewer starts once, moves to the configured display, enters fullscreen, signs in to UniFi Protect, and keeps retrying when the network or console is temporarily unavailable.

## Prepare Windows

1. Install all Windows updates and the current Microsoft Edge WebView2 runtime.
2. Create a local standard user named `Camera Display`.
3. Keep a separate administrator account for remote support.
4. Configure the display resolution and arrangement before configuring the viewer.
5. Set sleep to **Never** while plugged in. Configure display-off time for the store's operating requirements.
6. If company policy permits automatic Windows sign-in, configure it only for the standard `Camera Display` account.

Do not use an administrator account as the unattended camera account.

## Configure the viewer

1. Install and launch Unifi Protect Viewer while signed in as `Camera Display`.
2. Enter a descriptive store/profile name.
3. Paste the full UniFi Protect dashboard or liveview URL.
4. Enter the dedicated UniFi camera-viewer account credentials.
5. Select **Test Connection**. Resolve network or address errors before continuing.
6. Select **Save** and confirm that the camera wall loads.
7. Press **Ctrl+Shift+F10**, open **Startup**, and confirm:
   - The store profile is selected.
   - **Start Cameras with Windows** is enabled.
   - **Reconnect Automatically** is enabled.
   - **Start in Fullscreen** is enabled.
   - The correct target display is selected.
8. Restart Windows and confirm the camera wall appears without assistance.

Use a dedicated least-privilege UniFi account that can only view the required cameras.

## On-site recovery

- **F9** restarts the viewer.
- **F10** opens profile selection when multiple camera walls exist.
- **Ctrl+Shift+F10** opens technician configuration and support.
- **F11** toggles fullscreen.
- If loading takes longer than 20 seconds, the viewer shows **Retry now** and **Settings**, and retries automatically after 15 seconds.
- In **Support**, use **Restart Viewer**, **Open Log File**, or **Create Support Report**.

The support report excludes passwords. Passwords stored by the Windows build are protected with Windows DPAPI and can only be decrypted by the Windows user that configured the viewer.

## Remote-support checklist

Ask the on-site user to open **Ctrl+Shift+F10 → Support** and provide:

- Viewer version
- Whether startup is enabled
- Number of configured profiles
- The generated `support-report.txt`
- The `viewer.log` file if requested

Then check, in order:

1. Is the PC connected to the store network?
2. Can it reach the UniFi console address and port?
3. Is the UniFi console online?
4. Is the dedicated viewer account still enabled?
5. Does the configured liveview still exist?
6. Did the monitor numbering change?

## Acceptance test

Before leaving the site:

- Restart Windows and verify automatic fullscreen camera loading.
- Disconnect and reconnect Ethernet; verify automatic recovery.
- Restart the UniFi console or temporarily block access; verify the recovery screen and retry.
- Press F9, F10, Ctrl+Shift+F10, and F11 and verify each recovery shortcut.
- Launch the viewer twice and verify only one window remains.
- Confirm the standard store user cannot view saved passwords in the storage file.
- Run continuously for at least 72 hours before broad rollout.

## Windows kiosk policy

Start with the standard-user plus autostart configuration above because it remains easy to support remotely. For stricter deployments, Windows Assigned Access can provide a restricted experience. Windows Shell Launcher is appropriate only where the Windows edition and organizational management policy support replacing Explorer with a desktop application.
