---
title: Resources as Components
pull_requests: [20934]
---

Resources are very similar to Components: they are both data that can be stored in the ECS and queried.
The only real difference between them is that querying a resource will return either one or zero resources, whereas querying for a component can return any number of matching entities.

Even so, resources and components have always been separate concepts within the ECS.
This leads to some annoying restrictions.
While components have [`ComponentHooks`](https://docs.rs/bevy/latest/bevy/ecs/component/struct.ComponentHooks.html), it's not possible to add lifecycle hooks to resources.
The same is true for relations, observers, and a host of other concepts that already exist for components.
Moreover, the engine internals contain a lot of duplication because of it.

This motivates us to transition resources to components, and while most of the public API will stay the same, some breaking changes are inevitable.

The largest change is with regards to `ReflectResource`, which now shadows `ReflectComponent` exactly. When using `ReflectResource`, keep that in mind. The second largest change is that it's no longer possible to simultaneously derive `Component` and `Resource` on a struct. So

```rust
// 0.17.0
#[derive(Component, Resource)]
struct Dual
```
becomes
```rust
// 0.18.0
#[derive(Component)]
struct DualComp;

#[derive(Resource)]
struct DualRes;
```

It's still possible to doubly derive `#[reflcet(Component, Resource)]`, but since `ReflectResource` shadows `ReflectComponent` this isn't useful.

Moreover, `World::entities().len()` now gives more entities than you might expect.
For example, a new world no longer contains zero entities.
This is mostly important for unit tests.
If there is any place you are currently using `world.entities().len()`, we recommend you instead use a query `world.query<RelevantComponent>().query(&world).count()`.

Lastly, since `MapEntities` is implemented by default for components, it's no longer necessary to add `derive(MapEntities)` to a resource.

```rust
// 0.17.0
#[derive(Resource, MapEntities)]
struct EntityStruct(#[entities] Entity);

// 0.18.0
#[derive(Resource)]
struct EntityStruct(#[entities] Entity);
```
