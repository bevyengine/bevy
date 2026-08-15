---
title: "`TextFont::default()` font size is now `FontSize::Rem(1.)`"
pull_requests: [25231]
---

`TextFont::default()` now uses `FontSize::Rem(1.)` instead of `FontSize::Px(20.)`, so that the
`RemSize` resource actually sets the default font size. With the default `RemSize` of 20 logical
pixels this renders identically, but text left at the default size now scales when `RemSize`
changes. To keep a fixed size, set it explicitly:

```rust
// 0.20
TextFont {
    font_size: FontSize::Px(20.),
    ..default()
}
```
