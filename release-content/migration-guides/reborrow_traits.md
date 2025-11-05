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

Bevy 0.18 adds a few new traits to the ECS family: `ReborrowQueryData` and `ReborrowSystemParam`,
which allow for shortening the lifetime of a borrowed query item or system param respectively.
While not a breaking change, they're recommended to implement for most custom types where possible.

```rust
/// A [`SystemParam`] whose lifetime can be shortened via
/// [`reborrow`](ReborrowSystemParam::reborrow)-ing. This should be implemented
/// for most system params, except in the case of non-covariant lifetimes.
pub trait ReborrowSystemParam: SystemParam {
    /// Returns a `SystemParam` item with a smaller lifetime.
    fn reborrow<'wlong: 'short, 'slong: 'short, 'short>(
        item: &'short mut Self::Item<'wlong, 'slong>,
    ) -> Self::Item<'short, 'short>;
}
```
