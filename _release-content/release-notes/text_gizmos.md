---
title: "Text Gizmos"
authors: ["@ickshonpe", "@nuts-rice"]
pull_requests: [22732, 23120]
---

*TODO: Add a screenshot showing text gizmos rendered in a 3D scene.*

Gizmos can now render text using a built-in stroke font, with support for coloring
individual sections of text independently.

## Usage

Use `text` and `text_2d` to draw single-color text:

```rust
fn draw_text(mut gizmos: Gizmos) {
    gizmos.text_2d(
        Isometry2d::IDENTITY, // Position and rotation of the text
        "Hello Bevy",         // Only supports ASCII text
        40.0,                 // Font size in pixels
        Vec2::ZERO,           // Anchor point, zero is centered
        Color::WHITE,         // Color of the text
    );
}
```

Use `text_sections` and `text_sections_2d` to color each section of characters independently:

```rust
fn draw_colored_text(mut gizmos: Gizmos) {
    gizmos.text_sections(
        Isometry3d::IDENTITY,
        &[("Hello ", Color::WHITE), ("World!", Color::srgb(1., 0.3, 0.))], //Sections of text paired with color
        25.,
        Vec2::ZERO,
    );
}
```

Unlike Bevy's existing `Text2D` solution for worldspace text (damage numbers, nameplates, labels), this is *strictly* intended for dev tools and quick debugging.
The font is both very limited and non-configurable; its value is entirely in how easy it is to just stick some text on the screen.
