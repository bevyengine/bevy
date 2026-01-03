---
title: System Combinators
pull_requests: [20671]
---

`CombinatorSystem`s can be used to combine multiple `SystemCondition`s with logical operators (such as `and`, `or`, and `xor`). Previously, these combinators would propagate any errors made when running the combined systems:

```rust
// 0.17
#[derive(Component)]
struct Foo;

// This run condition will fail validation because there is not an entity with `Foo` in the world.
fn fails_validation(_: Single<&Foo>) -> bool {
    // ...
}

fn always_true() -> bool {
    true
}

let mut world = World::new();

// Because `fails_validation` is invalid, trying to run this combinator system will return an
// error.
assert!(world.run_system_once(fails_validation.or(always_true)).is_err());
```

This behavior has been changed in Bevy 0.18. Now if one of the combined systems fails, it will be considered to have returned `false`. The error will not be propagated, and the combinator logic will continue:

```rust
// 0.18
let mut world = World::new();

// `fails_validation` is invalid, but it is converted to `false`. Because `always_true` succeeds,
// the combinator returns `true`.
assert_eq!(matches!(world.run_system_once(fails_validation.or(always_true)), Ok(true)));
```

This affects the following combinators:

| Combinator | Rust Equivalent |
|:----------:|:---------------:|
| `and`      | `a && b`        |
| `or`       | `a \|\| b`      |
| `xor`      | `a ^ b`         |
| `nand`     | `!(a && b)`     |
| `nor`      | `!(a \|\| b)`   |
| `xnor`     | `!(a ^ b)`      |
