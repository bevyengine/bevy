---
title: "`AutoExposure` meters absolute scene luminance"
pull_requests: []
---

`AutoExposure` now meters absolute scene luminance in EV100, with the camera's `Exposure` divided
out. `AutoExposure::range` and the x-axis of `AutoExposureCompensationCurve` were in log2 of the
values stored in the HDR main texture. To convert them, add the camera's `Exposure::ev100` plus
`log2(9.6)` (about 3.26):

```rust
// With `Exposure::default()` (EV100 9.7) the offset is about 12.96.
let offset = exposure.ev100 + 9.6_f32.log2();
let range = (old_min + offset)..=(old_max + offset);
let curve = AutoExposureCompensationCurve::from_curve(LinearSpline::new([
    vec2(-4.0 + offset, -2.0),
    vec2(4.0 + offset, 2.0),
]));
```

The compensation (the y-axis) is unchanged. The default `range` is now `-8.0..=20.0`.

Adaptation runs on real time (`Time<Real>`) instead of virtual time, so it continues while virtual
time is paused and doesn't speed up or slow down with `Time<Virtual>`'s relative speed.

A newly added `AutoExposure` starts at its first metered exposure instead of adapting from the
camera's `Exposure`, so code that hides that initial fade, such as holding the screen black for a
few seconds after spawning a camera, is no longer needed.

`AutoExposureCompensationCurve`'s reflected `min_log_lum` and `max_log_lum` fields are now
`min_ev100` and `max_ev100`.
