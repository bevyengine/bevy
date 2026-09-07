---
title: "TextReader changes to support inline boxes"
pull_requests: [27510]
---

`TextReader::iter` and `get` now return items as `(Entity, usize, TextLayoutItem)`. `TextLayoutItem` is an enum with `Text` and `InlineBox` variants. The `InlineBox` variant refers to space created using the new `InlineBox` component.


`TextReader`'s `text`, `font`, `color`, `line_height`, `letter_spacing`, and their safe `get_*` equivalents have been removed. Instead use `TextReader::get` and match on the returned `TextLayoutItem`:

```rust
let text = match reader.get(root_entity, index) {
    Some((_, _, TextLayoutItem::Text { text, .. })) => Some(text),
    _ => None,
};
```
