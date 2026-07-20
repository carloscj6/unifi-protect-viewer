(function () {
  const invoke = (command, args) => window.__TAURI_INTERNALS__.invoke(command, args);
  window.require = function (module) {
    if (module !== 'electron') throw new Error('Unsupported preload module: ' + module);
    return {
      contextBridge: {
        exposeInMainWorld: (key, value) => {
          window[key] = value;
        },
      },
      ipcRenderer: {
        send: (channel, ...args) => {
          invoke('ipc', { channel, args }).catch(console.error);
        },
        invoke: (channel, ...args) => invoke('ipc', { channel, args }),
      },
    };
  };
})();
