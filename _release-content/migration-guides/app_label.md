---
title: "Further constraints on `AppLabel`"
pull_requests: [23377]
---

`AppLabel` needs some extra constraints

Before:

```rust,ignore
#[derive(Default, Debug, Clone, Copy, Hash, PartialEq, Eq, AppLabel)]
struct MyAppLabel;
```

After:

```rust,ignore
#[derive(Default, Debug, Clone, Copy, Hash, PartialEq, Eq, AppLabelInterior)]
struct MyAppLabel;

impl AppLabel for MyAppLabel {}
```
