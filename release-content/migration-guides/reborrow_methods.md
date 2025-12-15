---
title: ECS reborrowing methods
pull_requests: [22025]
---

Bevy 0.18 adds a new `reborrow` method to `QueryData` and `SystemParam`, which
enables shortening the lifetime of a system param/query item.

```rust
fn reborrow<'a>(item: &'a mut Self::Item<'_, '_>) -> Self::Item<'a, 'a>;
```

Since most implementations will have covariant lifetimes, this should be
an easy method to add. However, there's a couple narrow exceptions.

If you have a `ParamSet` in a custom system param that looks like
`ParamSet<'w, 's, InnerParam<'w, 's>>`, this is actually *invariant* over
the lifetimes `'w` and `'s`, so it's impossible to implement `reborrow`
for the custom param. Instead, you should write the inner param's lifetimes
as `'static`. For more info on lifetime variance, see the [nomicon](https://doc.rust-lang.org/nomicon/subtyping.html).
