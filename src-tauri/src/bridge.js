(function () {
  const invoke = (command, args) => window.__TAURI_INTERNALS__.invoke(command, args);
  // Electron's ipcRenderer.send delivers messages in call order. Tauri invoke
  // calls are promises and can otherwise race (save -> select -> launch).
  let sendQueue = Promise.resolve();

  function send(channel, args) {
    const operation = sendQueue.then(() => invoke('ipc', { channel, args }));
    // Keep the queue usable after a rejected command while still returning the
    // rejection to the caller that initiated it.
    sendQueue = operation.catch((error) => {
      console.error('[upv bridge] IPC send failed:', channel, error);
    });
    return operation;
  }

  function request(channel, args) {
    // Reads made after writes must observe those writes.
    return sendQueue.then(() => invoke('ipc', { channel, args }));
  }

  window.require = function (module) {
    if (module !== 'electron') throw new Error('Unsupported preload module: ' + module);
    return {
      contextBridge: {
        exposeInMainWorld: (key, value) => {
          window[key] = value;
        },
      },
      ipcRenderer: {
        send: (channel, ...args) => send(channel, args),
        invoke: (channel, ...args) => request(channel, args),
      },
    };
  };
})();
