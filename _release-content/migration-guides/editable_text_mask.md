---
title: "`EditableText` has new `mask` and `mask_state` fields"
pull_requests: [25117]
---

`EditableText` has two new public fields for password-style masking:
`mask: Option<char>` and `mask_state: bevy::text::MaskState`. Struct
literals without `..default()` no longer compile. Add `..default()`,
or set the new fields to their defaults (which preserve the previous
behavior):

```rust
EditableText {
    // ...
    mask: None,
    mask_state: MaskState::default(),
}
```

Patterns that destructure `EditableText` need the new fields or `..`.
