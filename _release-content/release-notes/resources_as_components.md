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

This first implementation is limited, only enabling observers for resources.

```rust
#[derive(Resource)]
struct GlobalSetting;

fn on_add_setting(add: On<Add, GlobalSetting>, query: Query<&LevelSetting>) {
    // ...
}
```

The main drawbacks are twofold. First it's no longer to derive both `Component` and `Resource` for a struct.
Secondly `ReflectResource` has been gutted, use `ReflectComponent` instead.
For more information, see the migration guide.
