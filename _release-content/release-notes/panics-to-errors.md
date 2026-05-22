---
title: Catching Panics
authors: ["@SpecificProtagonist"]
pull_requests: [24240]
---

For long-running programs, crashing can be unacceptable. If, for example, there is a bug in one of your image editor's tools, it's better for that tool to fail or to produce wrong results than to lose all your unsaved work.

Bevy's systems, commands and observers are able to return errors. You can either set an error handler case-by-case, or let the `FallbackErrorHandler` deal with it. But this used to only work for explicitly returned errors: Panics used to bring down the entire app.

In Bevy 0.xx, these panics now get turned into errors and passed to the fallback error handler. By default this re-panics, but now you can choose whether to log an error and continue, or whatever else you want.
