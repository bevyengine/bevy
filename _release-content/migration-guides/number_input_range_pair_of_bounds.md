---
title: "`NumberInputRange` now accepts two `Bound<T>`s"
pull_requests: [24636, 24701]
---

Previously, `NumberInputRange`s, used with `FeathersNumberInput` to specify input ranges inside `HardLimit` and `SoftLimit` components, accepted a `Range<T>`. However, the range was always treated as an inclusive range.

Now, it accepts two `Bound<T>`s to allow for greater flexibility in defining the start and end of your number input. `Bound::Excluded` is now respected. Use `Bound::Included` for your range end if you require an inclusive end.

```rust
// BEFORE
bsn! {
    @FeathersNumberInput
    HardLimit(NumberInputRange::I32(0..10))
    // 10 was still an acceptable number to reach via scrubbing and input.
}

// AFTER
bsn! {
    @FeathersNumberInput
    HardLimit(NumberInputRange::I32(Bound::Included(0), Bound::Excluded(10)))
    // 10 will no longer be reachable via scrubbing or input.
}
```

The `HardLimit::f32`, `HardLimit::f64`, ... and equivalent `SoftLimit` convenience methods now take an `impl RangeBounds<T>` to make it easier to use:

```rust
// Equivalent to code above
bsn! {
    @FeathersNumberInput
    HardLimit::i32(0..10)
    // 10 is not reachable via scrubbing.
}
```

This code now works and specifies an inclusive range:

```rust
bsn! {
    @FeathersNumberInput
    HardLimit::i32(0..=10)
    // 10 is reachable via scrubbing and input.
}
```
