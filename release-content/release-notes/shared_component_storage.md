---
title: Shared component storage
authors: ["@eugineerd"]
pull_requests: [19153, 19456]
---

Components can now be stored in `Shared` storage, which is a new memory-efficient storage type for immutable components.

Each unique component value with this storage type fragments the archetype, which means that all entities within the archetype are guaranteed to have the same value for that component.
This allows to store the component's value once for all entities, not only within the same `Archetype`, but also the `World`.
Since `Shared` components are immutable, modifying them for an entity requires to move it to a different archetype, which means that remove/insert performance of shared components is similar to `SparseSet` components.
On the other hand, iteration performance is more similar to `Table` components because the component value has to be retrieved only once per archetype.
To make it possible to store and compare these values efficiently, the components must implement the following traits: `Clone`, `Hash`, `PartialEq`, `Eq`.

```rs
#[derive(Component, Clone, Hash, PartialEq, Eq)]
#[component(storage = "Shared")]
enum Faction {
  Player,
  Enemy,
  Custom(u32),
}
```

Overall, this storage type is useful for components that are expected to have a limited number of possible values that can exist during the runtime of the app.
