const webdriverio = require('webdriverio');
const androidOptions = require('../helpers/caps').androidOptions;
const app = require('../helpers/apps').androidApiDemos;
const assert = require('chai').assert;

androidOptions.capabilities.app = app;

const MAX_TIMEOUT_FOR_TEST = 1_200_000;

describe('Create Android session', function () {
  this.timeout(MAX_TIMEOUT_FOR_TEST);
  let client;

  before(async function () {
    client = await webdriverio.remote(androidOptions);
  });

  afterEach(async function () {
    await client.deleteSession();
  });

  it('can start the example app', async function () {
    const res = await client.status();
    assert.isObject(res.build);

    await new Promise(r => setTimeout(r, 10000));
    await client.saveScreenshot('./android.png');

    const current_package = await client.getCurrentPackage();
    assert.equal(current_package, 'org.bevyengine.example');

  });
});
