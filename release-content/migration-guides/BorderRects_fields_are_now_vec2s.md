---
title: "`BorderRect` now has `Vec2` fields"
pull_requests: [21581]
---

The directional BorderRect fields (`left`, `right`, `top`, and `bottom`) have been replaced with `min_inset` and `max_inset` `Vec2` fields.

Using `min_inset` and `max_inset` removes the need to interpret `top` or `bottom` relative to the coordinate system, so the same logic will work consistently in both UI and 2D.
