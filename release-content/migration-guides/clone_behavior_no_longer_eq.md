---
title: `CloneBehavior` is no longer `PartialEq` or `Eq`
pull_requests: [18393]
---

`CloneBehavior` no longer implements `PartialEq` or `Eq` and thus does not work with the `==` and `!=` operators, as the internal
comparisons involve comparing function pointers which may result in unexpected results.

Use pattern matching to check for equality instead:

```rust
// 0.16
if clone_behavior == CloneBehavior::Ignore {
   ...
}

// 0.17
if matches!(clone_behavior, CloneBehavior::Ignore) {
   ...
}
```
