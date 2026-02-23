---
title: "Text Gizmos"
authors: ["@ickshonpe"]
pull_requests: [22732]
---

Gizmos can now render text using a built-in stroke font.

## usage

Use the `text` and `text_2d` to draw text using line segments:

```rust
fn draw_text(mut gizmos: Gizmos) {
    gizmos.text_2d(
        Isometry2d::IDENTITY, // Position and rotation of the text
        "Hello Bevy",         // Only supports ASCII text
        40.0,                 // Font size in pixels
        Vec2::ZERO,           // Anchor point, zero is centered
        Color::WHITE,         // Color of the text.
    );
}
```
