---
title: Nested query access
pull_requests: [21557]
---

Queries are now able to access data from multiple entities in the same query item.
This will be used to support richer querying across relations,
such as by querying components from an entity's parent.

However, some query operations are not sound for queries that access multiple entities,
and need additional trait bounds to ensure they are only used soundly.

An `IterQueryData` bound has been added to iteration methods on `Query`:

* `iter_mut`/ `iter_unsafe` / `into_iter`
* `iter_many_unique_mut` / `iter_many_unique_unsafe` / `iter_many_unique_inner`
* `get_many_mut` / `get_many_inner` / `get_many_unique_mut` / `get_many_unique_inner`
* `par_iter_mut` / `par_iter_inner` / `par_iter_many_unique_mut`
* `single_mut` / `single_inner`

`iter`, `iter_many`, `par_iter`, and `single` have no extra bounds,
since read-only queries are always sound to iterate.
`iter_many_mut` and `iter_many_inner` methods have no extra bounds, either,
since they already prohibit concurrent access to multiple entities.

In addition, a `SingleEntityQueryData` bound has been added to

* The `EntityRef::get_components` family of methods
* The `Traversal` trait
* The `Query::transmute` and `Query::join` families of methods
* The `QueryIter::sort` family of methods

All existing query types will satisfy those bounds, but generic code may need to add bounds.

```rust
// 0.17
fn generic_func<D: QueryData>(query: Query<D>) {
    for item in &mut query { ... }
}
// 0.18
fn generic_func<D: IterQueryData>(query: Query<D>) {
    for item in &mut query { ... }
}
```

Conversely, manual implementations of `QueryData` may want to implement `IterQueryData` and `SingleEntityQueryData` if appropriate.

Finally, two new methods have been added to `WorldQuery`: `init_nested_access` and `update_archetypes`.
Manual implementations of `WorldQuery` should implement those methods as appropriate.
Queries that only access the current entity may leave them empty,
but queries that delegate to other implementations, especially generic ones,
should delegate the new methods as well.
