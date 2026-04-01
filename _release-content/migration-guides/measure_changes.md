---
title: "`Measure` changes"
pull_requests: [23568]
---

`Measure::measure` no longer takes a separate `style: &taffy::Style` parameter. Instead the taffy `Style` is now accessible via a new `style` field on `MeasureArgs`.

The `width` and `height` fields of `MeasureArgs` have been renamed to `known_width` and `known_height`, respectively.
