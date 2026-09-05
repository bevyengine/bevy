---
title: Opt-in `ScreenSpaceTransmission`
pull_requests: [25201]
---

`ScreenSpaceTransmission` is no longer a required component of `Camera3d` so it's disabled by default.
This may change how your scenes with translucent or transparent materials are rendered!
Now you should add `ScreenSpaceTransmission` to the `Camera3d` to enable screen space specular transmission on it.
