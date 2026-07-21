'use strict';

/**
 * @file login.js
 * @description Auto-login handler – REFERENCE COPY ONLY.
 *
 * Inlined into src/js/preload.js. Edit here for review, then sync to preload.js.
 */

/**
 * Fills in the username and password fields and submits the login form.
 * Waits until the login button is available in the DOM before proceeding.
 *
 * @param {{ username: string, password: string }} credentials
 * @returns {Promise<void>}
 */
async function performLogin(credentials) {
  const usernameSelector =
    'input[name="username"], input[autocomplete="username"], input[type="email"], #user';
  const passwordSelector =
    'input[name="password"], input[autocomplete="current-password"], input[type="password"], #password';
  const enabledSubmit = () =>
    [...document.querySelectorAll('button[type="submit"]')].find((button) => !button.disabled);

  console.log('[upv] waiting for login form');
  const formReady = await waitUntil(
    () => document.querySelector(usernameSelector) || document.querySelector(passwordSelector),
    20_000,
  );
  if (!formReady) throw new Error('The UniFi login form did not appear');

  const usernameInput = document.querySelector(usernameSelector);
  let passwordInput = document.querySelector(passwordSelector);
  if (usernameInput) {
    console.log('[upv] filling username');
    setReactInputValue(usernameInput, credentials.username);
  }

  const rememberLogin = document.querySelector('#shouldSaveLogin');
  if (rememberLogin && !rememberLogin.checked) {
    simulateClick(rememberLogin);
  }

  if (!passwordInput) {
    const usernameSubmitReady = await waitUntil(() => Boolean(enabledSubmit()), 5_000);
    if (!usernameSubmitReady) throw new Error('The username step did not become ready');
    simulateClick(enabledSubmit());
    const passwordReady = await waitUntil(
      () => Boolean(document.querySelector(passwordSelector)),
      20_000,
    );
    if (!passwordReady) throw new Error('The password step did not appear');
    passwordInput = document.querySelector(passwordSelector);
  }

  console.log('[upv] filling password');
  setReactInputValue(passwordInput, credentials.password);
  const passwordSubmitReady = await waitUntil(() => Boolean(enabledSubmit()), 5_000);
  if (!passwordSubmitReady) throw new Error('The login button did not become ready');
  console.log('[upv] submitting login');
  simulateClick(enabledSubmit());
}

// (no module.exports – this file is a reference copy, not a runtime module)
