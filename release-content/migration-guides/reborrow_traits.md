---
title: ECS reborrowing traits
pull_requests: [22025]
---

Bevy 0.18 adds a new `reborrow` method to `QueryData`, which enables shortening the lifetime of a query item.

```rust
fn reborrow<'a>(item: &'a mut Self::Item<'_, '_>) -> Self::Item<'a, 'a>;
```

Since `QueryData` implementers already have to be covariant over their lifetimes,
this shouldn't make the trait any harder to implement. For most read-only query
data, the method can be implemented with a simple deref: `*item`.
