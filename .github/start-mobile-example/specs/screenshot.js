describe("Running Bevy Example", () => {
  it("can take a screenshot", async () => {
    // Sleep to wait for app startup, device rotation, ...
    await new Promise((r) => setTimeout(r, 5000));

    // Take local screenshot
    await browser.saveScreenshot("./screenshot.png");
  });
});
