---
title: "`AutoExposure` now includes pixels below its luminance range"
pull_requests: [25798]
---

`AutoExposure` now includes pixels below the minimum of `AutoExposure::range` when calculating average luminance, treating them as having the minimum luminance.
Previously, these pixels counted toward the percentages used by `AutoExposure::filter` but were skipped when calculating the average.

Scenes with a mix of very dark and brighter areas may now appear brighter, because dark pixels included in the average now pull it down.
If this changes the look of your scene, increase the lower end of `AutoExposure::filter` to leave out more of the darkest samples, or use `AutoExposure::metering_mask` to exclude specific areas of the screen.
