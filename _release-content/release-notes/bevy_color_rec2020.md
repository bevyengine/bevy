---
title: "Rec.2020 and wide-gamut color in `bevy_color`"
authors: ["@stuartparmenter"]
pull_requests: []
---

`bevy_color` now has a new `LinearRec2020` color space. You can read more on
[Rec2020 here](https://en.wikipedia.org/wiki/Rec._2020). It supports all color
operations that other spaces have, including conversion between spaces.

Wide-gamut and HDR colors can now be represented with this new space:

```rust
// A vivid Rec.2020 red, far outside the sRGB gamut:
let red = Color::rec2020(1.0, 0.0, 0.0);
// A Display P3 color, as shown in a macOS/CSS color picker:
let p3 = Color::display_p3(1.0, 0.2, 0.1);
// Any point in the CIE chromaticity diagram, in this case 5 times paper white
let bright = Color::cie_xy_y(0.3127, 0.3290, 5.0);
```
