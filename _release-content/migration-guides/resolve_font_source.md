---
title: "`resolve_font_source`"
pull_requests: [24378]
---

The `resolve_font_source` function has been removed. Use `FontSource::resolve_font_family` in its place.

```rust
// Old
let family = resolve_font_source(&text_font, fonts)?;

// New
let family = text_font.font.resolve_font_family(fonts)?;
```
