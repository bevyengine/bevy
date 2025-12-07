---
title: System Composition
authors: ["@ecoskey"]
pull_requests: [21811]
---

## `SystemRunner` SystemParam

We've been working on some new tools to make composing multiple ECS systems together
even easier. Bevy 0.18 introduces the `SystemRunner` `SystemParam`, allowing running
systems inside other systems!

```rust
fn count_a(a: Query<&A>) -> usize {
    a.count()
}

fn count_b(b: Query<&B>) -> usize {
    b.count()
}

let get_sum = (
    ParamBuilder::system(count_a),
    ParamBuilder::system(count_b)
)
.build_system(
    |mut run_a: SystemRunner<(), usize>, mut run_b: SystemRunner<(), usize>| -> Result<usize, RunSystemError> {
        let a = run_a.run()?;
        let b = run_b.run()?;
        Ok(a + b)
    }
);
```

## `compose!` and `compose_with!`

With this new API we've also added some nice macro syntax to go on top. The `compose!`
and `compose_with!` macros will automatically transform a provided closure, making
the new `SystemRunner` params seamless to use.

```rust
compose! {
    || -> Result<usize, RunSystemError> {
        let a = system!(count_a).run()?;
        let b = system!(count_b).run()?;
        Ok(a + b)
    }
}
```
