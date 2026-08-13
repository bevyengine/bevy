---
title: "RGB primaries and conversion matrices in `bevy_color`"
authors: ["@stuartparmenter"]
pull_requests: []
---

`bevy_color` now has a `primaries` module. It holds constants derived from ITU standards:

- `Chromaticity`, the CIE diagram coordinates.
- `RgbPrimaries`, with constants for `BT709`, `BT2020`, `DISPLAY_P3`, and `ACES_CG`.
- `rgb_to_rgb_matrix`, which derives a conversion matrix between any two primary sets.
