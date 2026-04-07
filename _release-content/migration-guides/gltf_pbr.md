---
title: "Invert `bevy_gltf` dependency with `bevy_pbr`"
pull_requests: [22569]
---

Previously, `bevy_gltf` depended on `bevy_pbr`. This meant scene definition was tightly coupled to rendering. This dependency has been inverted, to allow `bevy_gltf` to function without any of the rendering stack present.

`bevy_gltf` is also an optional dependency.

In 0.18, loading a material sub-asset would return a `Handle<StandardMaterial>`.

```rs
let handle: Handle<StandardMaterial> = asset_server.load("models/animated/Fox.glb#Material0");
```

In 0.19, loading a material sub-asset loads a `GltfMaterial` to accurately represent the data in the glTF file.
To load the `StandardMaterial`, use the `/std` suffix when the `bevy_pbr` feature is turned on (the feature is on by default).

```rs
let handle: Handle<GltfMaterial> = asset_server.load("models/animated/Fox.glb#Material0");
let handle_std: Handle<StandardMaterial> = asset_server.load("models/animated/Fox.glb#Material0/std");
```

You can disable PBR rendering by initializing `PbrPlugin` as so:

```rs
PbrPlugin {
    gltf_enable_standard_materials: false,
    ..Default::default()
}
```

`GltfExtensionHandler` trait's methods have been updated:

- `on_material` passes in the `material_asset : &GltfMaterial` and `material_label: &str`
- `on_spawn_mesh_and_material` also passes in the `material_label: &str`

`UvChannel` has moved from `bevy_pbr` to `bevy_mesh`.
