---
title: UI Gradients 
authors: ["@Ickshonpe"]
pull_requests: [18139, 19330, 19992]
---

Bevy now supports UI nodes that display a gradient that transitions smoothly between two or more colors.

You can now add the `BackgroundGradient` component to a `Node` to set its background to a gradient. If you also set a `BackgroundColor`, the background color is drawn first and the gradient(s) are drawn on top. You can also use the `BorderGradient` component to make the border use a gradient.

Both of these components wrap the `Gradient` enum type, which has three variants:`Linear`, `Conic` and `Radial`.

Each gradient type consists of the geometric properties for that gradient, a list of color stops, and the color space used for interpolation (Bevy defaults to using `InterpolationColorSpace::Oklab`).

```rust
commands.spawn((
    Node { width: px(20), height: px(20) },
    BackgroundGradient::from(LinearGradient {
        angle: 4.,
        stops: vec![
            ColorStop::new(Color::WHITE, percent(15)),
            ColorStop::new(Color::BLACK, percent(85)),
        ],
        ..default()
    })
))
```
