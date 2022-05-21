import { test, expect, Page } from '@playwright/test';

test.beforeEach(async ({ page }) => {
  await page.goto('http://localhost:8000/');
});

test.describe('WASM example', () => {
  test('Wait for success', async ({ page }, test_info) => {
    let start = new Date().getTime();

    let found = false;
    while (new Date().getTime() - start < 300_000) {
      let msg = await promise_with_timeout(100, on_console(page), "no log found");
      if (msg.includes("no log found")) {
        continue;
      }
      console.log(msg);
      if (msg.includes("Test successful")) {
        let prefix = process.env.SCREENSHOT_PREFIX === undefined ? "screenshot" : process.env.SCREENSHOT_PREFIX;
        await page.screenshot({ path: `${prefix}-${test_info.project.name}.png`, fullPage: true });
        found = true;
        break;
      }
    }

    expect(found).toBe(true);
  });

});

function on_console(page) {
  return new Promise(resolve => {
    page.on('console', msg => resolve(msg.text()));
  });
}

async function promise_with_timeout(time_limit, task, failure_value) {
  let timeout;
  const timeout_promise = new Promise((resolve, reject) => {
    timeout = setTimeout(() => {
      resolve(failure_value);
    }, time_limit);
  });
  const response = await Promise.race([task, timeout_promise]);
  if (timeout) {
    clearTimeout(timeout);
  }
  return response;
}