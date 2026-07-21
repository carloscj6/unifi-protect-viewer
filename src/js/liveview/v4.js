'use strict';

/**
 * @file v4.js
 * @description Liveview handler Protect 4.x / 5.x / 6.x / 7.x+ – REFERENCE COPY ONLY.
 *
 * All modern Protect versions (>= 4.x, including 7.x and beyond) share the
 * same UI structure introduced in 4.x and are handled identically here.
 * Future versions are supported automatically as long as the DOM selectors
 * remain stable.
 *
 * Legacy 3.x is handled by v3.js (isLegacyVersion3() guard in preload.js).
 *
 * Inlined into src/js/preload.js. Edit here for review, then sync to preload.js.
 */

const SEL = {
  fullscreenWrapper: '[class^=liveView__FullscreenWrapper]',
  liveViewWrapper: '[class^=liveView__LiveViewWrapper]',
  widgets: '[class^=dashboard__Widgets]',
  expandButton: 'button[class^=dashboard__ExpandButton]',
  dashboardContent: '[class^=dashboard__Content]',
  commonWidget: '[class^=common__Widget]',
  scrollable: '[class^=dashboard__Scrollable]',
  viewportWrapper: '[class^=liveview__ViewportsWrapper]',
  optionButtons: '[data-testid="option"]',
  timelineButtons:
    '[class^=LiveViewGridSlot__PlayerOptions] [class^=PlayerTopLeftControls__ButtonGroup]',
  viewportErrors: '[class^=ViewportError__Wrapper]',
};

/**
 * Strips the Unifi 4.x+ UI chrome and expands the liveview fullscreen.
 * Called twice (with a 4 s gap) because some elements render lazily.
 * @returns {Promise<void>}
 */
async function applyLiveviewV4andNewer() {
  // Wait for the fullscreen wrapper to appear
  await waitUntil(() => hasElements(document.querySelectorAll(SEL.fullscreenWrapper)));

  await dismissAllModals();

  // Hide global chrome
  applyStyle(document.body, 'background', 'black');
  applyStyle(document.getElementsByTagName('header')[0], 'display', 'none');
  applyStyle(document.getElementsByTagName('nav')[0], 'display', 'none');

  // Hide dashboard chrome
  applyStyle(document.querySelectorAll(SEL.widgets)[0], 'display', 'none');
  applyStyle(document.querySelectorAll(SEL.expandButton)[0], 'display', 'none');
  applyStyle(document.querySelectorAll(SEL.fullscreenWrapper)[0], 'backgroundColor', 'black');

  const contentEl = document.querySelectorAll(SEL.dashboardContent)[0];
  applyStyle(contentEl, 'display', 'block');
  applyStyle(contentEl, 'padding', '0');

  // Expand the viewport inside its wrapper
  const liveViewEl = document.querySelectorAll(SEL.liveViewWrapper)[0];
  applyStyle(liveViewEl?.querySelectorAll(SEL.commonWidget)[0], 'border', '0');
  applyStyle(liveViewEl?.querySelectorAll(SEL.scrollable)[0], 'paddingBottom', '0');
  applyStyle(
    liveViewEl?.querySelectorAll(SEL.viewportWrapper)[0],
    'maxWidth',
    'calc(100vh * 1.7777777777777777)',
  );

  // Hide the per-camera option overlay buttons (e.g. "remove camera")
  await waitUntil(() => hasElements(document.querySelectorAll(SEL.optionButtons)));
  document.querySelectorAll(SEL.optionButtons).forEach((btn) => applyStyle(btn, 'display', 'none'));

  // Hide the "Go to Timeline" button group (graceful: wait max 1 s)
  await waitUntil(() => hasElements(document.querySelectorAll(SEL.timelineButtons)), 1_000);
  document
    .querySelectorAll(SEL.timelineButtons)
    .forEach((btn) => applyStyle(btn, 'display', 'none'));

  // Paint missing / error camera slots black so they blend with the background
  document
    .querySelectorAll(SEL.viewportErrors)
    .forEach((el) => applyStyle(el, 'backgroundColor', 'black'));
}

/** Enters UniFi's own live-view fullscreen layout if it is not active yet. */
async function enterUniFiFullscreen() {
  const revealControls = () => {
    const x = Math.round(window.innerWidth / 2);
    const y = Math.max(0, window.innerHeight - 24);
    const target = document.elementFromPoint(x, y) || document.body;
    target.dispatchEvent(
      new PointerEvent('pointermove', { bubbles: true, clientX: x, clientY: y }),
    );
    target.dispatchEvent(new MouseEvent('mousemove', { bubbles: true, clientX: x, clientY: y }));
  };
  const findButton = () =>
    [...document.querySelectorAll('button')].find((button) => {
      return [...button.querySelectorAll('path')].some((path) =>
        (path.getAttribute('d') || '').startsWith('M16 3H4a1 1 0 0 0-1 1v12'),
      );
    });

  let button = findButton();
  const found =
    Boolean(button) ||
    (await waitUntil(() => {
      try {
        revealControls();
      } catch (_error) {}
      button = findButton();
      return Boolean(button);
    }, 20_000));
  if (!found) return false;
  simulateClick(button || findButton());
  await wait(750);
  return true;
}

// (no module.exports – this file is a reference copy, not a runtime module)
