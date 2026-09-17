---
title: "ComponentInfo no longer stores the component ID"
pull_requests: [25774]
---

`ComponentInfo::id()` has been removed, and `ComponentInfo` no longer stores the component ID. Component IDs are already used as keys by the collections that store `ComponentInfo` instances; if you need both values, retain the `ComponentId` when
retrieving the `ComponentInfo` from the collection:

```rust
// 0.19
let info = components.get_info(component_id).unwrap();
let id = info.id();

// 0.20
let id = component_id;
let info = components.get_info(id).unwrap();
```

`Components::iter_registered()`, `World::inspect_entity`, `World::iter_resources`,
and `World::iter_resources_mut` now return iterators that include `ComponentId`
alongside `&ComponentInfo`, to retain access to the component ID with the component
info:

```rust
// 0.19
for info in components.iter_registered() {
    let id = info.id();
    // ...
}

for info in world.inspect_entity(entity)? {
    let id = info.id();
    // ...
}

for (info, ptr) in world.iter_resources() {
    let id = info.id();
    // ...
}

for (info, ptr) in world.iter_resources_mut() {
    let id = info.id();
    // ...
}

// 0.20
for (id, info) in components.iter_registered() {
    // ...
}

for (id, info) in world.inspect_entity(entity)? {
    // ...
}

for (id, info, ptr) in world.iter_resources() {
    // ...
}

for (id, info, ptr) in world.iter_resources_mut() {
    // ...
}
```
