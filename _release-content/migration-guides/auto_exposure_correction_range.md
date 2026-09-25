---
title: "`AutoExposure` has a new `correction_range` field"
pull_requests: [25799]
---

`AutoExposure` has a new `correction_range` field that limits the automatic exposure correction independently of the histogram's luminance range.
It defaults to `f32::MIN..=f32::MAX`, which preserves the previous behavior.

If you construct `AutoExposure` without `..Default::default()`, add `correction_range: f32::MIN..=f32::MAX`.
