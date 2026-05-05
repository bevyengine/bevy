---
title: "The old `bevy_scene` is now `bevy_world_serialization"
pull_requests: [23619, 23630]
---

In **Bevy 0.19** we landed a subset of Bevy's Next Generation Scene system (often known as BSN), which now lives in the `bevy_scene` / `bevy::scene` crate. However the old `bevy_scene` system still needs to stick around for a bit longer, as it provides some features that Bevy's Next Generation Scene system doesn't (yet!):

1. It is not _yet_ possible to write a World _to_ BSN, so the old system is still necessary for "round trip World serialization".
2. The GLTF scene loader has not yet been ported to BSN, so the old system is still necessary to spawn GLTF scenes in Bevy.

For this reason, we have renamed the old `bevy_scene` crate to `bevy_world_serialization`. If you were referencing `bevy_scene::*` or `bevy::scene::*` types, rename those paths to `bevy_world_serialization::*` and `bevy::world_serialization::*` respectively.

Additionally, to avoid confusion / conflicts with the new scene system, all "scene" terminology / types have been reframed as "world serialization":

- `Scene` -> `WorldAsset` (as this was always just a World wrapper)
- `SceneRoot` -> `WorldAssetRoot`
- `DynamicScene` -> `DynamicWorld`
  - `DynamicScene::from_scene` -> `DynamicWorld::from_world_asset`
- `DynamicSceneBuilder` -> `DynamicWorldBuilder`
- `DynamicSceneRoot` -> `DynamicWorldRoot`
- `SceneInstanceReady` -> `WorldInstanceReady`
- `SceneLoader` -> `WorldAssetLoader`
- `ScenePlugin` -> `WorldSerializationPlugin`
- `SceneRootTemplate` -> `WorldAssetRootTemplate`
- `SceneSpawner` -> `WorldInstanceSpawner`
- `SceneFilter` -> `WorldFilter`
- `SceneLoaderError` -> `WorldAssetLoaderError`
- `SceneSpawnError` -> `WorldInstanceSpawnError`

GLTF scene spawning is the most likely source of breakage for most people, as round trip world serialization is a relatively niche use case. For most people, the migration should be as simple as:

```rust
// before
commands.spawn(SceneRoot(asset_server.load("scene.gltf#Scene0")));

// after
commands.spawn(WorldAssetRoot(asset_server.load("scene.gltf#Scene0")));
```

We know this naming is a bit awkward. Once we port GLTF loading over to BSN (hopefully in the next release), you will be able to do cool stuff like this:

```rust
bsn! {
    :"scene.gltf#Scene0"
    Transform { position: Vec3 { x: 10. } }
}
```

This would set _just_ the `x` position in the GLTF scene root to `x`, patching on top of the position defined in the gltf scene. Cool!
