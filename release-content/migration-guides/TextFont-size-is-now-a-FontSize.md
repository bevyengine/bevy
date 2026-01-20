---
title: "TextFont's `font_size` field is now a `FontSize`"
pull_requests: [22614]
---

The font_size field of `TextFont` is now a `FontSize`.

The new `FontSize` enum has fields:

```rust
pub enum FontSize {
    /// Font Size in logical pixels.
    Px(f32),
    /// Font size as a percentage of the viewport width.
    Vw(f32),
    /// Font size as a percentage of the viewport height.
    Vh(f32),
    /// Font size as a percentage of the smaller of the viewport width and height.
    VMin(f32),
    /// Font size as a percentage of the larger of the viewport width and height.
    VMax(f32),
    /// Font Size relative to the value of the `RemSize` resource.
    Rem(f32),
}
```
