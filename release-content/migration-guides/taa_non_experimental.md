---
title: TAA is no longer experimental
pull_requests: [18349]
---

TAA is no longer experimental.

`TemporalAntiAliasPlugin` no longer needs to be added to your app to use TAA. It is now part of `DefaultPlugins`, via `AntiAliasingPlugin`.

`TemporalAntiAliasing` now uses `MipBias` as a required component in the main world, instead of overriding it manually in the render world.
