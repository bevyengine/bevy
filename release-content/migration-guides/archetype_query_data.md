---
title: "`ArchetypeQueryData` trait"
pull_requests: [21581]
---

To support richer querying across relations,
Bevy now supports query data that are not archetypal: the query can return entities based on conditions that do not exclusively involve the entity's archetype.

An example of non-archetypal filter is `Changed<C>`: the entity is filtered based on the archetype (having the component C) but also based on the change ticks of the component.

Code that requires queries to `impl ExactSizeIterator` may need to replace `QueryData` bounds with `ArchetypeQueryData`.

```rust
// 0.17
fn requires_exact_size<D: QueryData>(q: Query<D>) -> usize {
    q.into_iter().len()
}
// 0.18
fn requires_exact_size<D: ArchetypeQueryData>(q: Query<D>) -> usize {
    q.into_iter().len()
}
```

Manual implementations of `QueryData` will now need to provide the `IS_ARCHETYPAL` associated constant.
This will be `true` for most existing queries,
although queries that wrap other queries should delegate as appropriate.
In addition, queries with `IS_ARCHETYPAL = true` should implement `ArchetypeQueryData`.
