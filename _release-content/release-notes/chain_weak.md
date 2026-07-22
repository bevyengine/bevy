---
title: Weak system ordering with chain_weak
authors: ["@JMS55"]
pull_requests: []
---

Ordering large groups of systems with `.chain()` is convenient, but it can be
overly strict. If system set `X` is chained before system set `Y`, every system
in `X` must finish before *any* system in `Y` can start, even when the systems
involved never touch the same data. This often leaves worker threads idle while they
wait for a handful of stragglers at the end of a system set, a pattern that shows up
frequently in the render world.

The new `chain_weak()` function provides a looser alternative. Like `.chain()`,
it adds ordering constraints between successive elements, but those constraints
are emitted as "must start before" (start-to-start) dependencies rather than
"must finish before" ones. A later system may not begin until the earlier one has
begun, but it does not wait for the earlier one to finish, so the two can overlap.

```rust
schedule.configure_sets(
    (
        ExtractCommands,
        PrepareMeshes,
        ManageViews,
        Queue,
        PhaseSort,
        Prepare,
        Render,
        Cleanup,
        PostCleanup,
    )
        .chain_weak(),
);
```

When two weakly-ordered systems actually conflict on their data access, the
executor already prevents them from running at the same time, so the start-to-start
ordering makes the earlier one win: the later system waits until the earlier one
finishes. Non-conflicting systems, however, get to overlap for increased parallelism!

Two cases keep their regular finish-to-start ordering: an earlier system that
produces deferred effects such as `Commands` (so the later system observes them,
with an `ApplyDeferred` sync point inserted as usual), and exclusive systems (which
cannot overlap anything regardless).

Because the scheduler can only see accesses it tracks, dependencies expressed
through interior mutability on read-only accesses, global state, or other untracked
methods are **not** respected. Use `chain_weak` only when your systems don't rely
on such hidden ordering, otherwise stick with `chain`.
