---
title: ExtractComponentPlugin signature change
pull_requests: [19053]
---

Removed usuned second type argument of `ExtractComponentPlugin`

```rust
// 0.16
app.add_plugins(ExtractComponentPlugin::<MyComponent, _>::default());

// 0.17
app.add_plugins(ExtractComponentPlugin::<MyComponent>::default());
```
