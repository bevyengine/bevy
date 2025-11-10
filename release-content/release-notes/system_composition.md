---
title: System Composition
authors: ["@ecoskey"]
pull_requests: [todo]
---

## `SystemRunner` SystemParam

We've been working on some new tools to make composing multiple ECS systems together
even easier. Bevy 0.18 introduces the `SystemRunner` `SystemParam`, allowing running
systems inside other systems!

```rust
fn count_a(a: Query<&A>) -> u32 {
    a.iter().len()
}

fn count_b(b: Query<&B>) -> u32 {
    b.iter().len()
}

let get_sum = (
    ParamBuilder::system(count_a),
    ParamBuilder::system(count_b)
)
.build_system(
    |mut run_a: SystemRunner<(), u32>, mut run_b: SystemRunner<(), u32>| -> Result<u32, RunSystemError> {
        let a = run_a.run()?;
        let b = run_b.run()?;
        Ok(a + b)
    }
);
```

## `compose!` and `compose_with!`

With this new API we've also added some nice macro syntax to go on top. The `compose!`
and `compose_with!` macros will automatically transform a provided closure, making
the new `SystemRunner` params almost seamless to use.

```rust
compose! {
    || -> Result<u32, RunSystemError> {
        let a = run!(count_a)?;
        let b = run!(count_b)?;
        Ok(a + b)
    }
}
```
