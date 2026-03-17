---
title: Implement Letter Spacing
authors: ["@gregcsokas"]
pull_requests: [23380]
---

Bevy's text system now supports `LetterSpacing`, a new component that controls 
the spacing between characters in a text entity.

A new `LetterSpacing` component has been added to `bevy_text`, following the same 
pattern as the existing `LineHeight` component. It supports the following variants:

- `LetterSpacing::Px(f32)` — absolute spacing in pixels
- `LetterSpacing::Rem(f32)` — spacing relative to the root font size

The default value is `LetterSpacing::Px(0.0)`, which preserves the existing behavior.

Previously there was no way to control the spacing between characters in Bevy's text 
system. This is a common typographic need, tighter spacing for stylized headings, 
wider spacing for readability or decorative effects. The feature aligns Bevy's text 
capabilities with CSS `letter-spacing`.

Add `LetterSpacing` as a component to any text entity:
```rust
commands.spawn((
    Text::new("Hello, Bevy!"),
    TextFont {
        font_size: FontSize::Px(48.0),
        ..default()
    },
    LetterSpacing::Px(4.0),
));
```

Negative values are also supported, which bring characters closer together:
```rust
LetterSpacing::Px(-2.0)
```