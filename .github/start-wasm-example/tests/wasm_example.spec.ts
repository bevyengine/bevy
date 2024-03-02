import { test, expect } from '@playwright/test';

test.beforeEach(async ({ page }) => {
  await page.goto('http://localhost:8000/');
});

const MAX_TIMEOUT_FOR_TEST = 300_000;

test.describe('WASM example', () => {
  test('Wait for success', async ({ page }, testInfo) => {
    let start = new Date().getTime();

    let found = false;
    while (new Date().getTime() - start < MAX_TIMEOUT_FOR_TEST) {
      let msg = await promiseWithTimeout(100, onConsole(page), "no log found");
      if (msg.includes("no log found")) {
        continue;
      }
      console.log(msg);
      if (msg.includes("Test successful")) {
        let prefix = process.env.SCREENSHOT_PREFIX === undefined ? "screenshot" : process.env.SCREENSHOT_PREFIX;
        await page.screenshot({ path: `${prefix}-${testInfo.project.name}.png`, fullPage: true });
        found = true;
        break;
      }
    }

    expect(found).toBe(true);
  });

});

function onConsole(page) {
  return new Promise(resolve => {
    page.on('console', msg => resolve(msg.text()));
  });
}

async function promiseWithTimeout(timeLimit, task, failureValue) {
  let timeout;
  const timeoutPromise = new Promise((resolve, reject) => {
    timeout = setTimeout(() => {
      resolve(failureValue);
    }, timeLimit);
  });
  const response = await Promise.race([task, timeoutPromise]);
  if (timeout) {
    clearTimeout(timeout);
  }
  return response;
}