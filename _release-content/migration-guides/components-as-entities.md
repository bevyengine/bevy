---
title: "Components as Entities"
pull_requests: [24728]
---

- `ComponentId::new` now takes `Entity` as an argument instead of `usize`. For debugging, you can use `ComponentId::from_u32`.
- `ComponentId::index` was removed in favor of implementing `ContainsEntity`, call `ComponentId::entity` to get the underlying entity.
- `ComponentIdSet` is now an `EntityEquivalentHashSet` instead of a `FixedBitSet`.
  - `ComponentIdSet::is_clear` has changed to `ComponentIdSet::is_empty`.
  - `ComponentIdSet::difference` has changed to `-`, i.e.: `difference = set - other`.
  - `ComponentIdSet::intersection` has changed to `&`, i.e.: `intersection = set & other`.
  - `ComponentIdSet::union` has changed to `|`, i.e.: `union = set | other`.
  - Other methods have remained the same.
- `ComponentIds` has been removed. Instead of `ComponentIds`, `ComponentsRegistrator::new` now takes `EntityAllocator`, while `ComponentsQueuedRegistrator::new` now takes `RemoteAllocator`.
- `Access` and `EcsAccessType` no longer derive `Hash`.
- `ResourceEntities` was removed. The following methods have been removed with it: `World::resource_entities`, `EntityWorldMut::resource_entities`, `UnsafeWorldCell::resource_entities`. If you need the entity linked with a `ComponentId`, simply call `component_id.entity()`.
- Despawning a resource entity has been upgraded from a `warn!` to a `panic!`, moreover, removing `IsResource` from a resource entity also panics.

In 0.19, you could attach components to a resource by simply calling `world.spawn((Res1, Comp1, Comp2))`. In 0.20, this no longer works as `Res1` needs to be on the resource entity allocated by `world.register_component<Res1>()`. In 0.20, adding components looks as follows:

```rust
let entity = world.register_component::<R>().entity();
world.spawn_at(entity, (Res1, Comp1, Comp2));
```

Additionally, manually implementing `Resource` through

```rust
#[derive(Component, Default)]
struct R;

impl Resource for R {}
```

has become less viable, as now `Res` and `ResMut` panic when `IsResource` has not been made a required component for a resource.
Use `#[derive(Resource)]` instead.
