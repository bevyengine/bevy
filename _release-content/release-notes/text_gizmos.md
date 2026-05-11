---
title: "Text Gizmos"
authors: ["@ickshonpe", "@nuts-rice"]
pull_requests: [22732, 23120]
---

*TODO: Add a screenshot showing text gizmos rendered in a 3D scene.*

Sometimes you just want to slap a label on something while debugging.
Text gizmos are for exactly that: a zero-setup way to draw world-space text anywhere in your scene using a built-in stroke font.

Unlike Bevy's `Text2D` — the right choice for damage numbers, nameplates, and in-game labels — text gizmos are *strictly* for dev tools.
The font is fixed and only supports ASCII; the value is entirely in "text now plz".

Use `Gizmos::text` and `text_2d` to quickly draw text:

```rust
fn draw_text(mut gizmos: Gizmos) {
    gizmos.text_2d(
        Isometry2d::IDENTITY, // Position and rotation of the text in world-space
        "Hello Bevy",         // Only supports ASCII text
        40.0,                 // Font size in screen-space pixels
        Vec2::ZERO,           // Anchor point, zero is centered
        Color::WHITE,         // Color of the text
    );
}
```

If you want to color each section of characters separately, reach for `text_sections` and `text_sections_2d`.
