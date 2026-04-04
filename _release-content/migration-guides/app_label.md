---
title: "Further constraints on `AppLabel`"
pull_requests: [23377]
---

`AppLabel` needs some extra constraints

Before:

```rust,ignore
#[derive(AppLabel, Default, Debug, Clone, Copy, Hash, PartialEq, Eq)]
struct MyAppLabel;
```

After:

```rust,ignore
#[derive(AppLabelBase, Default, Debug, Clone, Copy, Hash, PartialEq, Eq)]
struct MyAppLabel;

impl AppLabel for MyAppLabel {}
```
