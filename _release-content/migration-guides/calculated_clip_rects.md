---
title: "`CalculatedClip` now stores transformed clip rectangles"
pull_requests: [24148]
---

`CalculatedClip` is now an enum with `Rects` and `FullyClipped` variants. 
- `Rects` is the list of the clipping regions inherited by a UI entity, each is defined by a `Rect` along with an `Affine2` transform.
- `FullyClipped` means that the UI entity is clipped completely and will neither be rendered nor pickable.

`CalculatedClip::contains_point` can be used to test whether a point in physical-pixel coordinates is clipped.