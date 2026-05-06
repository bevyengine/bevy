---
title: `CalculatedClip` now stores transformed clip rectangles
pull_requests: []
---

`CalculatedClip` is now an enum with `Rects` and `FullyClipped` variants. `Rects` holds a list of `Rect` in node local coords and `Affine2` world-to-local transform.