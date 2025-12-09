---
title: Contiguous access
authors: ["@Jenya705"]
pull_requests: [21984]
---

Enables accessing slices from tables directly via Queries.

## Goals

`QueryIter` has a new method `as_contiguous_iter`, which allows quering contiguously (i.e., over tables). For it to work the query data must implement `ContiguousQueryData` and the query filter `ArchetypeFilter`. When a contiguous iterator is used, the iterator will jump over whole tables, returning corresponding values. Some notable implementors of `ContiguousQueryData` are `&T` and `&mut T`, returning `&[T]` and `(&mut T, ContiguousComponentTicks<true>)` correspondingly, where the latter structure in the latter tuple lets you change update ticks. Some notable implementors of `ArchetypeFilter` are `With<T>` and `Without<T>` and notable structs not implementing it are `Changed<T>` and `Added<T>`.

This is for example useful, when an operation must be applied on a big amount of entities lying in the same tables, which allows for the compiler to auto-vectorize the code, thus speeding it up.

### Usage

`QueryIter::as_contiguous_iter` method returns an `Option<ContiguousQueryIter>`, which is only `None`, when the query is not dense (i.e., iterates over archetypes, not over tables).

```rust
fn apply_velocity(query: Query<(&Velocity, &mut Position)>) {
    // `as_contiguous_iter()` cannot ensure all invariants on the compilation stage, thus
    // when a component uses a sparse set storage, the method will return `None`
    for (velocity, (position, mut ticks)) in query.iter_mut().as_contiguous_iter().unwrap() {
        for (v, p) in velocity.iter().zip(position.iter_mut()) {
            p.0 += v.0;
        }
        // sets ticks, which is optional
        ticks.mark_all_as_updated();
    }
}
```
