---
title: "`ImageRenderTarget`s `scale_factor` field is now an `f32`"
pull_requests: [21054]
---

The `scale_factor` field on `ImageRenderTarget` is now an `f32` and no longer requires wrapping in `FloatOrd`.
