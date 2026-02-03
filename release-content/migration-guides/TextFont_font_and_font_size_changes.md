---
title: "Changes to `TextFont`'s `font_size` and `font` fields"
pull_requests: [22156, 22614]
---

`TextFont`'s `font` field has been changed from a `Handle<Font>` to a `FontSource`, and its `font_size` field has changed from an `f32` to a `FontSize`.

`FontSource` has two variants: `Handle`, which identifies a font by asset handle, and `Family`, which selects a font by its family name.

`FontSource` implements `From<Handle<Font>>`, migration of existing code should only require calling `into()` on the handle.

Font texture atlases are no longer automatically cleared when the font asset they were generated from is removed. This is because there is no way to remove individual fonts from cosmic text's `FontSystem`. So even after the asset is removed, the font is still accessible using the family name with `FontSource::family` and removing the text atlases naively could cause a panic as rendering expects them to be present.

For `font_size`, the migration is to wrap the `f32` value in a `FontSize::Px(...)`.

Concretely:

```rust
TextFont {
    font: asset_server.load("FiraMono-medium.ttf"),
    font_size: 35.,
    ..default()
}
```

becomes

```rust
TextFont {
    font: asset_server.load("FiraMono-medium.ttf").into(),
    font_size: FontSize::Px(35.),
    ..default()
}
```
