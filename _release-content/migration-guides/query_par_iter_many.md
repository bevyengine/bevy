---
title: "`Query::par_iter_many` methods only accept slices"
pull_requests: [26336]
---

The `par_iter_many`, `par_iter_many_unique`, and `par_iter_many_unique_mut` methods on `Query`
now only accept `&[E]` and `&UniqueEntityEquivalentSlice<E>` instead of `Iterator<Item = E>`.

These methods internally require a slice
to break up the list into chunks to iterate in parallel.
Previously, this was done by performing a `collect()` or `collect_set()`
to build a `Vec` or `UniqueEntityEquivalentVec`.
But that conversion was wasteful when the original collection was already a slice.

Callers that needed the `collect()` or `collect_set()`
will need to call it themselves before calling `par_iter`.
Remember to use `collect_set()` instead of `collect()`
when creating a `UniqueEntityVec` or other `UniqueEntityEquivalentVec`
from an `EntitySetIterator` to avoid the cost of checking for duplicates.

Callers passing a `Vec` or `UniqueEntityVec` by value will need to add `&` to convert it to a slice.

```rust
// 0.20
query.par_iter_many(iterator).for_each(...)
query.par_iter_many_unique_mut(iterator).for_each(...)
// 0.21
query.par_iter_many(&iterator.collect::<Vec<_>>()).for_each(...)
query.par_iter_many_unique_mut(&iterator.collect_set::<UniqueEntityEquivalentVec<_>>()).for_each(...)
```
