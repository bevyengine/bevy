---
title: Changes to Bevy's system parallelism strategy
pull_requests: [16885]
---

The scheduler will now prevent systems from running in parallel if there *could* be an archetype that they conflict on, even if there aren't actually any.
To expand on this, previously, the scheduler would look at the entities that exist to determine if there's any overlap.
Now, it determines it solely on the basis of the function signatures of the systems that are run.
This was done as a performance optimization: while in theory this throws away potential parallelism,
in practice, tests on engine and user code found that scheduling overhead dominates parallelism gains with the current multithreaded executor.
Swapping to this simpler, more conservative test allows us to speed up these checks and improve resource utilization.
There are more improvements planned here, so stay tuned!

To understand what this means, consider the following example. These systems will now conflict even if no entity has both `Player` and `Enemy` components:

```rust
fn player_system(query: Query<(&mut Transform, &Player)>) {}
fn enemy_system(query: Query<(&mut Transform, &Enemy)>) {}
```

To allow them to run in parallel, use `Without` filters, just as you would to allow both queries in a single system:

```rust
// Either one of these changes alone would be enough
fn player_system(query: Query<(&mut Transform, &Player), Without<Enemy>>) {}
fn enemy_system(query: Query<(&mut Transform, &Enemy), Without<Player>>) {}
```

If you encounter a performance regression in your application due to this change, *please* open an issue.
We would be very interested to understand your use case, see your benchmarking results, and help you mitigate any issues.
