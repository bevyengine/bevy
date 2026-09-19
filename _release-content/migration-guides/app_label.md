---
title: "Further constraints on `AppLabel`"
pull_requests: [23377]
---

`AppLabel` has some extra constraints. It now requires an implementation of `Default` and `Copy`.

To access `.intern()` on an `AppLabel`, you must now import the `AppLabelInterior` trait.

Before:

```rust,ignore
#[derive(Debug, Hash, PartialEq, Eq, AppLabel)]
struct MyAppLabel;
```

After:

```rust,ignore
#[derive(Default, Debug, Clone, Copy, Hash, PartialEq, Eq, AppLabel)]
struct MyAppLabel;
```
