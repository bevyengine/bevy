---
title: "`FontSource` generic family variants"
pull_requests: [24378]
---

The generic font family variants on `FontSource`, such as `FontSource::SansSerif` and `FontSource::Monospace`, have been replaced by a new `GenericFontFamily` enum. Use the corresponding `FontSource` constructor methods, or convert `GenericFontFamily` into a `FontSource`.

```rust
// Old
TextFont {
    font: FontSource::SansSerif,
    ..default()
}

// New
TextFont {
    font: FontSource::sans_serif(),
    ..default()
}
```

`FontCx::set_generic_family` now takes a `GenericFontFamily` instead of a `parley::GenericFamily`.
