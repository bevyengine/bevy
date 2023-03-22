const iosCaps = {
    platformName: 'iOS',
    automationName: 'XCUITest',
    deviceName: process.env.IOS_DEVICE_NAME || 'iPhone 6s',
    platformVersion: process.env.IOS_PLATFORM_VERSION || '12.1',
    orientation: 'LANDSCAPE',
    app: undefined
};

const androidCaps = {
    platformName: 'Android',
    automationName: 'UiAutomator2',
    deviceName: process.env.ANDROID_DEVICE_NAME || 'My Android Device',
    platformVersion: process.env.ANDROID_PLATFORM_VERSION || null,
    orientation: 'LANDSCAPE',
    app: undefined
};

const serverConfig = {
    path: '/wd/hub',
    host: process.env.APPIUM_HOST || 'localhost',
    port: process.env.APPIUM_PORT || 4723,
    logLevel: 'info',
    connectionRetryTimeout: 600_000
};

const androidOptions = Object.assign(
    {
        capabilities: androidCaps
    },
    serverConfig
);

const iosOptions = Object.assign(
    {
        capabilities: iosCaps
    },
    serverConfig
);


module.exports = {
    androidOptions,
    iosOptions
};
