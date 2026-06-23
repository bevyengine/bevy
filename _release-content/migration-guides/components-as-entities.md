---
title: "Components as Entities"
pull_requests: [ ... ]
---

- `ComponentId::new` now takes `Entity` as an argument instead of `usize`.
- `ComponentId::index` was removed.
- `ComponentId::from_u32` was added.
- `ComponentId` now implements `ContainsEntity` so the entity can be gotten through `ComponentId::entity`.
- `ComponentIdSet` is now an `EntityEquivalentHashSet` instead of a `FixedBitSet`. This means that methods like `union_with` no longer work, use `bitor_assign` instead.
- `ComponentIds` has been removed.
- `ComponentsRegistrator::new` now takes `EntityAllocator` instead of `ComponentIds`.
- `ComponentsQueuedRegistrator::new` not takes `RemoteAllocator` instead of `ComponentIds`.
- `Access` no longer derives `Hash`.
- `EcsAccessType` no longer derives `Hash`.
