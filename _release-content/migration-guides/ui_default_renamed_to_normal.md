---
title: "`AlignItems::Default`, `JustifyItems::Default`, `AlignContent::Default`, and `JustifyContent::Default` are renamed to `::Normal``"
pull_requests: [26344]
---

To better match the CSS spec and taffy, we're renaming several enum variants to Normal.

`AlignItems::Default` is now `AlignItems::Normal`
`JustifyItems::Default` is now `JustifyItems::Normal`
`AlignContent::Default` is now `AlignContent::Normal`
`JustifyContent::Default` is now `JustifyContent::Normal`

`::DEFAULT` and `::default()` remain the same for all of these.

```rust
// 0.20
Node { align_items: AlignItems::Default, ..default() }

// 0.21
Node { align_items: AlignItems::Normal, ..default() }
```
