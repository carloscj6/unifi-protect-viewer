'use strict';

const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const test = require('node:test');
const vm = require('node:vm');

const bridgeSource = fs.readFileSync(
  path.join(__dirname, '..', '..', 'src-tauri', 'src', 'bridge.js'),
  'utf8',
);

function loadBridge(invoke) {
  const window = { __TAURI_INTERNALS__: { invoke } };
  const context = vm.createContext({ window, console });
  vm.runInContext(bridgeSource, context);
  return window.require('electron').ipcRenderer;
}

test('Tauri bridge serializes sends in Electron delivery order', async () => {
  const calls = [];
  let releaseFirst;
  const firstPending = new Promise((resolve) => {
    releaseFirst = resolve;
  });
  const ipc = loadBridge(async (_command, payload) => {
    calls.push(payload.channel);
    if (payload.channel === 'profilesSave') await firstPending;
    return null;
  });

  const save = ipc.send('profilesSave', [{ id: 'profile-1' }]);
  const select = ipc.send('activeProfileSet', 'profile-1');
  const launch = ipc.send('launchProfile', 'profile-1');

  await Promise.resolve();
  assert.deepEqual(calls, ['profilesSave']);
  releaseFirst();
  await Promise.all([save, select, launch]);
  assert.deepEqual(calls, ['profilesSave', 'activeProfileSet', 'launchProfile']);
});

test('Tauri bridge waits for pending writes before reads', async () => {
  const calls = [];
  let releaseWrite;
  const writePending = new Promise((resolve) => {
    releaseWrite = resolve;
  });
  const ipc = loadBridge(async (_command, payload) => {
    calls.push(payload.channel);
    if (payload.channel === 'profilesSave') await writePending;
    if (payload.channel === 'profilesLoad') return [{ id: 'profile-1' }];
    return null;
  });

  const write = ipc.send('profilesSave', [{ id: 'profile-1' }]);
  const read = ipc.invoke('profilesLoad');
  await Promise.resolve();
  assert.deepEqual(calls, ['profilesSave']);
  releaseWrite();
  assert.deepEqual(await read, [{ id: 'profile-1' }]);
  await write;
  assert.deepEqual(calls, ['profilesSave', 'profilesLoad']);
});

test('a failed send does not permanently block the queue', async () => {
  const calls = [];
  const ipc = loadBridge(async (_command, payload) => {
    calls.push(payload.channel);
    if (payload.channel === 'profilesSave') throw new Error('write failed');
    return null;
  });

  await assert.rejects(ipc.send('profilesSave', []), /write failed/);
  await ipc.send('activeProfileSet', 'profile-1');
  assert.deepEqual(calls, ['profilesSave', 'activeProfileSet']);
});
