---
title: "`AppLabel` refactor"
pull_requests: []
---

The `AppLabel` derive has been updated

```rust,ignore
#[derive(Default, Debug, Clone, Copy, Hash, PartialEq, Eq, AppLabel)]
struct MyAppLabel;
```

After:

```rust,ignore
#[app_label]
struct MyAppLabel;
```
