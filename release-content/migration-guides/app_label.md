---
title: "`AppLabel` refactor"
pull_requests: [23377]
---

The `AppLabel` derive has been updated

```rust,ignore
#[derive(AppLabel, Default, Debug, Clone, Copy, Hash, PartialEq, Eq)]
struct MyAppLabel;
```

After:

```rust,ignore
use bevy_derive::app_label;

#[app_label]
struct MyAppLabel;
```
