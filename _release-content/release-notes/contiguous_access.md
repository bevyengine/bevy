---
title: Contiguous query access
authors: ["@Jenya705"]
pull_requests: [21984, 24181]
---

[SIMD] is a critical modern tool for performance optimization, but using it in Bevy has always been harder than it needed to be.
Table components in Bevy are already laid out flat in memory — all `Transform` components are stored as values in a contiguous table, exactly what SIMD wants.
The `Query` iterator just wasn't exposing that structure: it handed you one entity's component at a time, and the compiler had no way to know the underlying data was a contiguous array.

`contiguous_iter` and `contiguous_iter_mut` hand you the whole table slice at once. LLVM can see the contiguous array and auto-vectorize — or you can reach for explicit SIMD yourself.

On a bulk `position += velocity` update over 10,000 entities, this gives some serious speedups:

| Method                          | Time    | Time (AVX2) |
| ------------------------------- | ------- | ----------- |
| Normal iteration                | 5.58 µs | 5.51 µs     |
| Contiguous iteration            | 4.88 µs | 1.87 µs     |
| Contiguous, no change detection | 4.40 µs | 1.58 µs     |

If your project has CPU-heavy workloads (physics engines are a prime example), you should try this out immediately.

```rust
fn apply_health_decay(mut query: Query<(&mut Health, &HealthDecay)>) {
    for (mut health, decay) in query.contiguous_iter_mut().unwrap() {
        for (h, d) in health.iter_mut().zip(decay) {
            h.0 *= d.0;
        }
    }
}
```

The `contiguous_iter` family of methods only returns `Ok` if the query is dense. That means:

- All of the fetched components must use the default "table" storage strategy.
- The query filters cannot disrupt the returned query data. "Archetypal filters" like `With<T>` and `Without<T>` are fine; `Changed<T>` and `Added<T>` are not, since they require a per-entity check that makes it impossible to return raw table slices.

Because these conditions are fixed properties of the query type, you're safe to unwrap here unless you are writing generic code,
or working with dynamic components.

You may have noticed that the table above had *three* rows.
While change detection is a generally useful feature, it does incur measurable performance overhead.
By default, `contiguous_iter_mut` returns `ContiguousMut<T>`.
Just like the ordinary `Mut<T>`, it triggers change detection automatically on dereference.
If you don't care about that, `bypass_change_detection()` gives you the raw `&mut [T]` directly for even faster access.
Vroom!

[SIMD]: https://en.wikipedia.org/wiki/Single_instruction,_multiple_data
