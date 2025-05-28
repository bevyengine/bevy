---
title: UI Gradients 
authors: ["@Ickshonpe"]
pull_requests: [18139]
---

Support for UI node's that display a gradient that transitions smoothly between two or more colors.

To draw a UI node with a gradient insert the components `BackgroundGradient` and `BorderGradient`, which both newtype a vector of `Gradient`s. If you set a background color, the background color is drawn first and the gradient(s) are drawn on top.

The are three gradient structs corresponding to the three types of gradients supported: `LinearGradient`, `ConicGradient` and `RadialGradient`. These are then wrapped by the `Gradient` enum discriminator which has `Linear`, `Conic` and `Radial` variants.

Each gradient type consists of the geometric properties for that gradient and a list of color stops.
Color stops consist of a color, a position or angle and an optional hint. If no position is specified for a stop, it's evenly spaced between the previous and following stops. Color stop positions are absolute. With the list of stops:

```rust
vec![vec![ColorStop::new(RED, Val::Percent(90.), ColorStop::new(Color::GREEN, Val::Percent(10.))
```

the colors will be reordered and the gradient will transition from green at 10% to red at 90%.

Colors are interpolated between the stops in SRGB space. The hint is a normalized value that can be used to shift the mid-point where the colors are mixed 50-50 between the stop with the hint and the following stop.

For sharp stops with no interpolated transition, place two stops at the same point.

`ConicGradients` and `RadialGradients` have a center which is set using the new `Position` type. `Position` consists of a normalized (relative to the UI node) Vec2 anchor point and a responsive x, y offset.
