---
title: DynamicSceneBuilder and DynamicScene::from_scene now require a &TypeRegistry
pull_requests: [23401]
---

Previously, `DynamicSceneBuilder` and `DynamicScene` would get the type registry out of the world
being extracted. However, when building a world from scratch just for serialization, this required
artificially cloning the registry and putting it in the world being saved.

Now, `DynamicSceneBuilder` and `DynamicScene::from_scene` require an existing type registry. For
example, before:

```rust
let world: &World = ...;
let scene = DynamicSceneBuilder::from_world(world)
    .extract_entity(e1)
    .extract_entity(e2)
    .extract_resources()
    .build();
```

Becomes:

```rust
let world: &World = ...;
let scene = {
    let type_registry = world.resource::<AppTypeRegistry>().read();
    DynamicSceneBuilder::from_world(world, &type_registry)
        .extract_entity(e1)
        .extract_entity(e2)
        .extract_resources()
        .build()
};
```

For `DynamicScene::from_scene`, before:

```rust
let type_registry: AppTypeRegistry = get_from_main_world();
let mut scene: Scene = ...;
// Previously the scene world needed the type registry.
scene.world.insert_resource(type_registry);
let dynamic_scene = DynamicScene::from_scene(scene);
```

Becomes:

```rust
let type_registry: AppTypeRegistry = get_from_main_world();
let scene: Scene = ...;
// No need to insert into the scene!
let dynamic_scene = DynamicScene::from_scene(scene, &type_registry.read());
```
