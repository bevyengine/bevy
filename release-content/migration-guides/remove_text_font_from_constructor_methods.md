---
title: Replaced `TextFont` constructor methods with `From` impls
pull_requests: [20335, 20450]
---

The `TextFont::from_font` and `TextFont::from_line_height` constructor methods have been removed in favor of `From` trait implementations.

```rust
// 0.16
let text_font = TextFont::from_font(font_handle);
let text_font = TextFont::from_line_height(line_height);

// 0.17
let text_font = TextFont::from(font_handle);
let text_font = TextFont::from(line_height);
```
