'use strict';

const assert = require('node:assert');
const fs = require('node:fs');
const path = require('node:path');
const test = require('node:test');

const root = path.join(__dirname, '..', '..');
const rust = fs.readFileSync(path.join(root, 'src-tauri', 'src', 'lib.rs'), 'utf8');
const pages = [
  'src/html/config.html',
  'src/html/index.html',
  'src/html/profile-select.html',
  'src/html/debug-block.js',
].map((file) => fs.readFileSync(path.join(root, file), 'utf8'));
const preload = fs.readFileSync(path.join(root, 'src/js/preload.js'), 'utf8');
const sharedCss = fs.readFileSync(path.join(root, 'src/html/shared.css'), 'utf8');

test('connection screen calls the native Rust command directly', () => {
  assert.match(pages[0], /window\.__TAURI__\.core\.invoke\('connection_test'/);
  assert.match(rust, /#\[tauri::command\]\s*fn connection_test/);
});

test('profile writes are awaited before native navigation', () => {
  assert.match(pages[0], /await window\.__TAURI__\.core\.invoke\('profiles_save'/);
  assert.match(pages[0], /await window\.__TAURI__\.core\.invoke\('launch_profile'/);
});

test('all page commands are registered native handlers', () => {
  const used = new Set(
    pages.flatMap((source) =>
      [...source.matchAll(/core\.invoke\('([a-z_]+)'/g)].map((match) => match[1]),
    ),
  );
  for (const command of used) {
    assert.match(rust, new RegExp(`fn ${command}\\b`), `${command} must have a Rust handler`);
    assert.match(rust, new RegExp(`\\b${command},?`), `${command} must be registered`);
  }
});

test('retired compatibility runtime is absent from application sources', () => {
  const legacyName = 'elec' + 'tron';
  const forbidden = [legacyName, legacyName + 'API', 'ipc' + 'Renderer', 'context' + 'Bridge'];
  const sources = [rust, ...pages, preload];
  for (const token of forbidden) {
    assert.ok(sources.every((source) => !source.toLowerCase().includes(token.toLowerCase())));
  }
});

test('persistent WebView session drives login and camera page lifecycles', () => {
  assert.match(rust, /app_local_data_dir\(\)\?[\s\S]*join\("EBWebView"\)/);
  assert.match(rust, /\.data_directory\(webview_data_dir\)/);
  assert.match(preload, /if \(isLoginPage\(\)\)[\s\S]*await performLogin\(config\);[\s\S]*return;/);
  assert.match(preload, /window\.location\.href = config\.url;\s*return;/);
  assert.match(
    preload,
    /if \(isMfaPage\(\)\)[\s\S]*hideOverlay\('MFA input required'\);[\s\S]*return;/,
  );
  assert.match(preload, /#shouldSaveLogin/);
  assert.match(preload, /startCameraPage\(\);/);
  assert.doesNotMatch(preload, /scheduleSessionRenewal/);
});

test('successful Protect navigation forces the native camera window fullscreen', () => {
  assert.match(
    rust,
    /PageLoadEvent::Finished[\s\S]*path\(\)\.contains\("\/protect\/"\)[\s\S]*window\.set_fullscreen\(true\)/,
  );
});

test('modern camera view clicks the UniFi internal fullscreen control', () => {
  assert.match(rust, /window\.eval\("startCameraPage\(\)"\)/);
  assert.match(
    preload,
    /const fullscreenTask = enterUniFiFullscreen\(\)[\s\S]*loader-screen[\s\S]*await fullscreenTask/,
  );
  assert.match(preload, /startsWith\('M16 3H4a1 1 0 0 0-1 1v12'\)/);
  assert.match(preload, /new PointerEvent\('pointermove'/);
  assert.match(preload, /new MouseEvent\('mousemove'/);
  assert.match(preload, /simulateClick\(button \|\| findButton\(\)\)/);
});

test('F enters and Escape exits native and UniFi fullscreen', () => {
  assert.match(preload, /event\.key\.toLowerCase\(\) === 'f'/);
  assert.match(preload, /invokeTauri\('set_fullscreen', \{ fullscreen: true \}\)/);
  assert.match(preload, /event\.key === 'Escape'/);
  assert.match(preload, /exitUniFiFullscreen\(\)/);
  assert.match(preload, /invokeTauri\('set_fullscreen', \{ fullscreen: false \}\)/);
  assert.match(rust, /fn set_fullscreen[\s\S]*window\.set_fullscreen\(fullscreen\)/);
});

test('system idle restores fullscreen after one minute', () => {
  assert.match(rust, /GetLastInputInfo/);
  assert.match(rust, /system_idle_seconds\(\) < 60/);
  assert.match(rust, /window\.show\(\)/);
  assert.match(rust, /window\.unminimize\(\)/);
  assert.match(rust, /window\.set_fullscreen\(true\)/);
  assert.match(rust, /window\.set_focus\(\)/);
  assert.match(rust, /window\.eval\("enterUniFiFullscreen\(\)\.catch\(\(\) => \{\}\)"\)/);
  assert.doesNotMatch(preload, /invokeTauri\('fullscreen_idle_state'\)/);
});

test('closing the window keeps the native idle watchdog running', () => {
  assert.match(rust, /WindowEvent::CloseRequested/);
  assert.match(rust, /api\.prevent_close\(\)/);
  assert.match(rust, /window\.hide\(\)/);
  assert.match(rust, /viewer closed to background/);
});

test('local screens use the minimal UniFi blue and white theme', () => {
  assert.match(sharedCss, /--accent:\s*#006fff/i);
  assert.match(sharedCss, /--bg-secondary:\s*#ffffff/i);
  assert.match(sharedCss, /--bg-card:\s*#ffffff/i);
  assert.match(sharedCss, /--text-primary:\s*#17212f/i);
});
