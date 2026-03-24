---
title: "`ComponentId` is now backed by `u32`"
pull_requests: [23497]
---

`ComponentId::new()` now takes `u32` instead of `usize`.

Before:

```rust
let id = ComponentId::new(3usize);
```

After:

```rust
let id = ComponentId::new(3u32);
```
