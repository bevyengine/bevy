---
title: Separate Border Colors
pull_requests: [18682]
---

The `BorderColor` struct now contains separate fields for each edge, `top`, `bottom`, `left`, `right`. To keep the existing behavior, replace `BorderColor(color)` with `BorderColor::all(color)`, and `border_color.0 = new_color` with `*border_color = BorderColor::all(new_color)`.
