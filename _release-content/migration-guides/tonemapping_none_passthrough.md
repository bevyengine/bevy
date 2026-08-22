---
title: "`Tonemapping::None` is now a full passthrough"
pull_requests: [25499]
---

`Tonemapping::None` now leaves the image completely untouched. `ColorGrading` and
`DebandDither` stop applying.

If you use `Tonemapping::None` with `ColorGrading` or `DebandDither`, switch to the
new `Tonemapping::Linear`. It applies no tone curve but keeps grading and dither
working exactly as they do under the named operators.

`Tonemapping::None` also stops clamping negative color channels to zero. If your
scene relies on that clamp, `Tonemapping::Linear` keeps it.
