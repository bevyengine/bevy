---
title: "`Tonemapping::None` is now a full passthrough"
pull_requests: [25499]
---

`Tonemapping::None` is now a full passthrough. `ColorGrading` and `DebandDither` no longer
apply under it, and negative color channels are no longer clamped to zero. `Camera3d`
enables `DebandDither` by default, so a `Camera3d` without `Hdr` that used
`Tonemapping::None` renders differently. If you used `Tonemapping::None` to turn off the
tone curve, use the new `Tonemapping::Linear` instead. It applies no tone curve and keeps
grading, dither, and the clamp.

```rust
// 0.19
commands.spawn((Camera3d::default(), Tonemapping::None));

// 0.20
commands.spawn((Camera3d::default(), Tonemapping::Linear));
```

`Camera2d` defaults to `Tonemapping::None`, so a `Camera2d` with `DebandDither::Enabled` or
`ColorGrading` now renders without them. Add `Tonemapping::Linear` to keep the 0.19 result.
