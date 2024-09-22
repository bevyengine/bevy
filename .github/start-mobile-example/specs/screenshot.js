const percyScreenshot = require('@percy/appium-app');


describe('Running Bevy Example', () => {
  it('can take a screenshot', async () => {

    // Sleep to wait for app startup, device rotation, ...
    await new Promise(r => setTimeout(r, 2000));

    // Take local screenshot
    await browser.saveScreenshot('./screenshot.png');

    // Take screenshot for visual testing
    await percyScreenshot(`Bevy Mobile Example`);

  });
});
