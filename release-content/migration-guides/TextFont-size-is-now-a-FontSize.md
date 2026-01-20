---
title: "TextFont's `font_size` field is now a `FontSize`"
pull_requests: [22614]
---

The `font_size` field on `TextFont` has changed from `f32` to `FontSize`.
Existing code should wrap values in `FontSize::Px(...)`.
