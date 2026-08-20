---
title: "`with_luminance` no longer clamps in `bevy_color`"
pull_requests: [25394]
---

`LinearRgba::with_luminance` now scales the components to the target luminance and does
not clamp the result, so HDR and wide-gamut values survive. A saturated color or a target
above 1.0 can produce components outside `[0.0, 1.0]`. Operations that convert through it,
such as `Srgba::with_luminance` and `Color::with_luminance`, change the same way.
The `Laba` to `Lcha` conversion now retains the color's full gamut.

If you were using these methods to bring colors back into SDR range, you'll now need
to clamp explicitly with `c.red.clamp(0., 1.)`. Alternatively, convert through
`ColorToPacked`, which still quantizes colors to [0, 1].
