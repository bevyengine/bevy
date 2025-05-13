---
title: ExtractComponentPlugin signature change
pull_requests: [19053]
---

Removed unused second type argument of `ExtractComponentPlugin`

```rust
// was
app.add_plugins(ExtractComponentPlugin::<MyComponent, _>::default());

// now
app.add_plugins(ExtractComponentPlugin::<MyComponent>::default());
```
