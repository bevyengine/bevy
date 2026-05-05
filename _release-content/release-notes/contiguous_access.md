---
title: Contiguous access
authors: ["@Jenya705"]
pull_requests: [21984]
---

Enables accessing slices from tables directly via Queries.

## Goals

`Query` and `QueryState` have new methods `contiguous_iter`, `contiguous_iter_mut` and `contiguous_iter_inner`, which allows querying contiguously (i.e., over tables). For it to work the query data must implement `ContiguousQueryData` and the query filter `ArchetypeFilter`. When a contiguous iterator is used, the iterator will jump over whole tables, returning corresponding data. Some notable implementors of `ContiguousQueryData` are `&T` and `&mut T`, returning `&[T]` and `ContiguousMut<T>` correspondingly, where the latter structure lets you get a mutable slice of components as well as corresponding ticks. Some notable implementors of `ArchetypeFilter` are `With<T>` and `Without<T>` and notable types **not implementing** it are `Changed<T>` and `Added<T>`.

For example, this is useful, when an operation must be applied on a large amount of entities lying in the same tables, which allows for the compiler to auto-vectorize the code, thus speeding it up.

### Usage

`Query::contiguous_iter` and `Query::contiguous_iter_mut` return a `Option<QueryContiguousIter>`, which is only `None`, when the query is not dense (i.e., iterates over archetypes, not over tables).

```rust
fn apply_velocity(query: Query<(&Velocity, &mut Position)>) {
    // `contiguous_iter_mut()` cannot ensure all invariants on the compilation stage, thus
    // when a component uses a sparse set storage, the method will return `None`
    for (velocity, mut position) in query.contiguous_iter_mut().unwrap() {
        // we could also have used position.bypass_change_detection() to do even less work.
        for (v, p) in velocity.iter().zip(position.iter_mut()) {
            p.0 += v.0;
        }
    }
}
```
