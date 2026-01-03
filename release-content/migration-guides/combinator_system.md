---
title: System Combinators
pull_requests: [20671]
---

The `CombinatorSystem`s can be used to combine multiple `SystemCondition`s with logical operators. Previously, the conditions would short circuit if the system failed to run, for example because it's query could not be filled by the world.

Now, the `CombinatorSystem`s will work as expected, following the semantics of rust's logical operators.
Namely, if a `SystemCondition` fails, it will be considered to have returned `false` and in combinators that don't short circuit the other condition will now be run.

Specifically, the combinators act as follows:

| Combinator | Rust Equivalent |
|:----------:|:---------------:|
| `and`      | `a && b`        |
| `or`       | `a \|\| b`      |
| `xor`      | `a ^ b`         |
| `nand`     | `!(a && b)`     |
| `nor`      | `!(a \|\| b)`   |
| `xnor`     | `!(a ^ b)`      |

```rust
fn vacant(_: crate::system::Single<&Vacant>) -> bool {
    true
}

fn is_true() -> bool {
    true
}

assert!(world.query::<&Vacant>().iter(&world).next().is_none());

// 0.17
assert!(world.run_system_once(is_true.or(vacant)).is_err());

// 0.18
assert!(matches!(world.run_system_once(is_true.or(vacant)), Ok(true)));
```
