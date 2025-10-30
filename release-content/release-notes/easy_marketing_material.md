---
title: Helpers to Produce Marketing Material
authors: ["@mockersf"]
pull_requests: [21235, 21237]
---

Bevy can take a screenshot of what's rendered since 0.11. This is now easier to setup to help you create marketing material, so that you can take screenshot with consistent formatting with the new `EasyScreenshotPlugin`. With its default settings, once you add this plugin to your application, a PNG screenshot will be taken when you press the `PrintScreen` key. You can change the trigger key, or the screenshot format between PNG, JPEG or BMP.

It is now possible to record a movie from Bevy, with the new `EasyScreenRecordPlugin`. This plugins add a toggle key, space bar by default, that will toggle screen recording. Recording can also be started and stopped programmatically with the `RecordScreen` messages.
