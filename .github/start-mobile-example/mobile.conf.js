exports.config = {
    user: process.env.BROWSERSTACK_USERNAME,
    key: process.env.BROWSERSTACK_ACCESS_KEY,

    updateJob: false,
    specs: [
        './specs/screenshot.js'
    ],
    exclude: [],

    capabilities: [{
        project: "Bevy Example",
        build: 'Bevy Example Runner',
        name: 'run_example',
        device: process.env.DEVICE || 'Samsung Galaxy S23',
        os_version: process.env.OS_VERSION || "13.0",
        app: process.env.BROWSERSTACK_APP_ID,
        'browserstack.debug': true,
        orientation: 'LANDSCAPE'
    }],

    logLevel: 'info',
    coloredLogs: true,
    screenshotPath: './screenshots/',
    baseUrl: '',
    waitforTimeout: 10000,
    connectionRetryTimeout: 90000,
    connectionRetryCount: 3,

    framework: 'mocha',
    mochaOpts: {
        ui: 'bdd',
        timeout: 20000
    }
};