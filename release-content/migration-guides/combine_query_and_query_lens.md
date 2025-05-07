---
title: Combine `Query` and `QueryLens`
pull_requests: [18162]
---

The `QueryLens::query()` method has been deprecated.
The `QueryLens` type has now been combined with `Query`, so most methods can be called directly on the `QueryLens` and the call can simply be removed.
If that doesn't work and you do need a fresh `Query`, the call to `.query()` can be replaced with `.reborrow()`.

```rust
fn with_query(query: Query<&T>) {}
fn with_lens(lens: QueryLens<&T>) -> Result {
  // 0.16
  for item in lens.query().iter() {}
  with_query(lens.query());
  // 0.17
  for item in lens.iter() {}
  with_query(lens.reborrow());
}
```
