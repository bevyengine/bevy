---
title: Combine `Query` and `QueryLens`
pull_requests: [19787]
---

The `QueryLens::query()` method has been deprecated.
The `QueryLens` type has now been combined with `Query`, so most methods can be called directly on the `QueryLens` and the call can simply be removed.
If that doesn't work and you do need a fresh `Query`, the call to `.query()` can be replaced with `.reborrow()`.

```rust
fn with_query(query: Query<&T>) {}
fn with_lens(lens: QueryLens<&T>) -> Result {
  // 0.17
  for item in lens.query().iter() {}
  with_query(lens.query());
  // 0.18
  for item in lens.iter() {}
  with_query(lens.reborrow());
}
```

One consequence of this change is that `Query<'w, 's, D, F>` is no longer covariant in `'s`.
This means trying to convert a `&'a Query<'w, 's, D, F>` to a `&'a Query<'w, 'a, D, F>` will now fail with `lifetime may not live long enough`.

Note that `'w` is still covariant, so converting `&'a Query<'w, 's, D, F>` to `&'a Query<'a, 's, D, F>` will still succeed.

You can usually resolve that error by introducing a new lifetime parameter for `'s`,
although in many cases it will be simpler to use the `reborrow()` or `as_readonly()` methods to shorten the lifetimes and create an owned query.

```rust
// 0.17
struct HasQueryBorrow<'a> {
  query: &'a Query<'a, 'a, &'static C>,
}

fn create(query: &Query<&'static C>) {
  let hqb = HasQueryBorrow { query };
  //                         ^^^^^
  // This now fails with
  // error: lifetime may not live long enough
}

// 0.18 - Add additional lifetime parameter
struct HasQueryBorrow<'a, 's> {
  query: &'a Query<'a, 's, &'static C>,
}

fn create(query: &Query<&'static C>) {
  let hqb = HasQueryBorrow { query };
}

// 0.18 - Or store an owned query instead of a reference
// and use `reborrow()` or `as_readonly()`
struct HasQueryBorrow<'a> {
  query: Query<'a, 'a, &'static C>,
}

fn create(query: &Query<&'static C>) {
  let hqb = HasQueryBorrow { query: query.as_readonly() };
}
```
