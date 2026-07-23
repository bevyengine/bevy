---
title: Schedule randomization
authors: ["@andriyDev"]
pull_requests: [25094]
---

Before a schedule runs (and therefore, your systems), it first computes the order that systems run
in based on the ordering constraints (`.before()`, `.after()`, `.chain()`) of systems and system
sets. However, in addition to this, the schedule must also resolve **conflicts** - if system A and
system B both mutate component C, and there's no ordering between A and B, the schedule needs to
pick one to run first. So far, the rule has been that this is non-deterministic.

In practice though, schedules pick the order of these conflicting systems "deterministically, but
arbitrarily". Put simply, your systems might accidentally be in the right order, but making an
unrelated change to the graph might suddenly put it in the wrong order. This problem can be very
difficult to detect.

Introducing schedule randomization! This will randomize the order of systems while maintaining any
explicit system ordering constraints. Once the `debug` feature is enabled, `ScheduleBuildSettings`
will include a `shuffle_seed` field, that users can set to randomize their schedules. For example:

```rust
App::new()
    .add_plugins(DefaultPlugins)
    .edit_schedule(Update, |schedule| {
        // Make sure to add the `rand` crate with `cargo add rand`.
        let rng_seed: u64 = rand::random();
        // Consider logging out the seed, so you can reproduce the error if you find a bug!
        info!("Randomizing Update schedule with seed={rng_seed}");
        schedule.set_build_settings(ScheduleBuildSettings {
            shuffle_seed: Some(rng_seed),
            ..Default::default()
        });
    })
    .run();
```

This can be used for "property testing", to verify that your systems satisfy some property despite
different orderings of systems.

There are some caveats however. Currently, when using `auto_insert_apply_deferred`, systems with
commands are always placed before the earliest sync point they can. This means that although your
systems may not have the correct ordering, they might "accidentally" have the correct ordering
because of which sync point it uses. We hope to fix this in the future.

In addition, the multi-threaded executor executes systems greedily: it looks for the first
unexecuted system whose dependencies are finished and that has no other conflicting systems running.
The result is that even if the shuffle results in the order `(A, B, C)`, `C` could run before `B` if
`A` and `B` conflict.
