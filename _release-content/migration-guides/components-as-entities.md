---
title: "Components as Entities"
pull_requests: [24728]
---

- `ComponentId::new` now takes `Entity` as an argument instead of `usize`. For debugging, you can use `ComponentId::index`.
- `ComponentId::index` was removed in favor of implementing `ContainsEntity`, call `ComponentId::entity` to get the underlying entity.
- `ComponentIdSet` is now an `EntityEquivalentHashSet` instead of a `FixedBitSet`. This means that methods like `union_with` no longer work, use `bitor_assign` instead.
- `ComponentIds` has been removed. Instead of `ComponentIds`, `ComponentsRegistrator::new` now takes `EntityAllocator`, while` ComponentsQueuedRegistrator::new` now takes `RemoteAllocator`.
- `Access` and `EcsAccessType` no longer derive `Hash`.
- `ResourceEntities` was removed. The following methods have been removed with it: `World::resource_entities`, `EntityWorldMut::resource_entities`, `UnsafeWorldCell::resource_entities`. If you need the entity linked with a `ComponentId`, simply call `component_id.entity()`.
