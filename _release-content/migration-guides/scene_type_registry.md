---
title: DynamicSceneBuilder and DynamicScene::from_scene now require a &TypeRegistry
pull_requests: [23401]
---

Previously, `DynamicSceneBuilder` and `DynamicScene` (now `DynamicWorldBuilder` and `DynamicWorld` respectively) would get the type registry out of the world
being extracted. However, when building a world from scratch just for serialization, this required
artificially cloning the registry and putting it in the world being saved.

In 0.19 `DynamicWorldBuilder` and `DynamicWorld::from_world_asset` require an existing type registry in Bevy.
For example, before:

```rust
// 0.18
let world: &World = ...;
let scene = DynamicSceneBuilder::from_world(world)
    .extract_entity(e1)
    .extract_entity(e2)
    .extract_resources()
    .build();
```

Becomes:

```rust
// 0.19
let world: &World = ...;
let dynamic_world = {
    let type_registry = world.resource::<AppTypeRegistry>().read();
    DynamicWorldBuilder::from_world(world, &type_registry)
        .extract_entity(e1)
        .extract_entity(e2)
        .extract_resources()
        .build()
};
```

For `DynamicScene::from_scene`:

```rust
// 0.18
let type_registry: AppTypeRegistry = get_from_main_world();
let mut scene: Scene = ...;
// Previously the scene world needed the type registry.
scene.world.insert_resource(type_registry);
let dynamic_scene = DynamicScene::from_scene(scene);
```

Becomes:

```rust
// 0.19
let type_registry: AppTypeRegistry = get_from_main_world();
let world_asset: WorldAsset = ...; // Scene was renamed to WorldAsset in 0.19
// No need to insert into the world asset!
let dynamic_world = DynamicWorld::from_world_asset(&world_asset, &type_registry.read());
```
