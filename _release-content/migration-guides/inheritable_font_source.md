---
title: "`InheritableFont::font` is now a `FontSource`"
pull_requests: [26242]
---

The `font` field of `bevy_feathers`'s `InheritableFont` is now a `FontSource` instead of a `Handle<Font>`, so it can name a font family list with per-glyph fallback.

In `bsn!`, wrap a font path in `FontSourceTemplate::Handle`:

```rust
// Before
InheritableFont {
    font: fonts::REGULAR,
    ..
}

// After
InheritableFont {
    font: FontSourceTemplate::Handle(fonts::REGULAR),
    ..
}
```

To match the built-in Feathers widgets, use `FontSourceTemplate::Families(fonts::SANS)` instead.
When you construct the component directly, convert the handle with `.into()`.
