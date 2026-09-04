---
title: "Several built-in schedules now order their system sets weakly"
pull_requests: [25128]
---

A number of Bevy's built-in schedules previously ordered their top-level system sets with
`.chain()`, a "must finish before" ordering: every system in a set had to
finish before *any* system in the next set could start. These sets are now ordered with the new
`.chain_weak()` (and in a few places `.after_weak()`/`.before_weak()`), which keeps an
ordering only between systems whose data accesses actually conflict and leaves
non-conflicting systems in adjacent sets unordered, letting them run in any order for better
parallelism.

The affected orderings are:

- **`Render` schedule** — the `RenderSystems` sets (`ExtractCommands`, `PrepareMeshes`, `Queue`,
  `Prepare`, `Render`, `Cleanup`, `PostCleanup`, …) and the `PrepareResources*`,
  `QueueMeshes`/`QueueSweep`, and asset-preparation sub-orderings within them.
- **`RenderGraph` schedule** — the `RenderGraphSystems` sets (`Begin`, `Render`, `Submit`,
  `Finish`).
- **`Core2d` and `Core3d` schedules** — the pass sets (`Prepass`, `MainPass`, `EarlyPostProcess`,
  `PostProcess`).
- **`ExtractSchedule`** — the UI extract sets (`RenderUiSystems`), `MeshExtractionSystems`
  (now `after_weak(extract_visibility_ranges)`), and `DirtySpecializationSystems` (now ordered
  with `before_weak`).
- **`PostUpdate`** — the UI `UiSystems` sets (`CameraUpdateSystems`, `Prepare`, `Propagate`,
  `Content`, `Layout`, `PostLayout`).

For most users this changes nothing. When two weakly-ordered systems actually conflict on their
tracked data access, a normal ordering is kept between them so the earlier one
still runs first. Deferred-effect producers (`Commands`) keep their sync-point ordering, and
exclusive systems are treated as always conflicting. In practice almost every ordering above is
still enforced this way — the change only relaxes ordering between systems that have no data
dependency at all.

However, if you added a custom system to one of these sets and relied on a system in an earlier
set *finishing* before yours *starts* — where that dependency is **not** expressed through tracked
ECS access (for example, communication through interior mutability on a `Res<T>`, a channel, an
atomic, or global/`NonSend` state) — that ordering is no longer guaranteed. Restore a strict
ordering explicitly:

```rust
// Before: relied on the implicit strict ordering between these render sets.
app.add_systems(Render, my_system.in_set(RenderSystems::Prepare));

// After: request the strict ordering you need explicitly.
app.add_systems(
    Render,
    my_system
        .in_set(RenderSystems::Prepare)
        .after(some_earlier_system),
);
```

Prefer expressing the dependency through the ECS (have the producer write a `ResMut<T>` and the
consumer read `Res<T>`) so the scheduler can see it and order the systems for you.
