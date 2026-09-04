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

`Camera2d` now defaults to `Tonemapping::Linear`. No change is needed for 2D cameras. A
`Camera2d` with `Hdr` now runs the tonemapping pass.

Bevy logs a warning for a camera that combines `Tonemapping::None` with `DebandDither::Enabled`
or a non-default `ColorGrading`.
