---
title: "New Font features"
authors: ["@ickshonpe"]
pull_requests: [22156]
---

`TextFont` has been expanded to include new fields:

```rust
pub struct TextFont {
    pub font: FontSource,
    pub font_size: FontSize,
    pub weight: FontWeight,
    pub width: FontWidth,
    pub style: FontStyle,
    pub font_smoothing: FontSmoothing,
    pub font_features: FontFeatures,
}
```

FontSource has two variants: Handle, which identifies a font by asset handle, and Family, which selects a font by its family name.

`FontWidth` is a newtype struct representing OpenType font stretch classifications ranging from `ULTRA_CONDENSED` (50%) to `ULTRA_EXPANDED` (200%).

`FontStyle` is an enum used to set the slant style of a font, with variants `Normal`, `Italic`, or `Oblique`.

The system font support is very basic for now. You load them using the `CosmicFontSystem` resource:

```rust
font_system.db_mut().load_system_fonts()
```

Then they are available to be selected by family name using `FontSource::Family`.

The `font_size` field is now a `FontSize`, enabling responsive font sizing.

`FontSize` is an enum with variants:

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

`Rem` units are currently resolved using `RemSize`, which is a new `Resource`. `RemSize` just newtypes an `f32` currently.

`Text2d`'s support for viewport coords is limited. A `Text2d` entity's resolved font size is always based on the size of the primary window, not on the size of its render target(s).
