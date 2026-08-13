---
title: "HDR-safe clamping in `bevy_color` luminance operations"
pull_requests: []
---

`with_luminance`, `lighter`, and `darker` now preserve HDR and wide-gamut values
instead of clamping the result to `[0.0, 1.0]`. Colors inside that range behave
exactly as before. The `Laba` to `Lcha` conversion now retains the color's full gamut.

If you were using these methods to bring colors back into SDR range, you'll now need
to clamp explicitly with `c.red.clamp(0., 1.)`. Alternatively, convert through
`ColorToPacked`, which still quantizes colors to [0, 1].
