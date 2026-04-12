---
title: Rename `Font::try_from_bytes` to `Font::from_bytes`
pull_requests: [22879, 23777]
---

`Font::try_from_bytes` has been renamed to `Font::from_bytes` to reflect that it no longer returns `Result`.

```rust
// 0.18
let font = Font::try_from_bytes(bytes.to_vec()).unwrap();

// 0.19
let font = Font::from_bytes(bytes.to_vec(), "MyFontFamily");
```

Note that the family name is not part of this specific change, but is required in 0.19.
