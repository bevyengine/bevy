---
title: "Invert `bevy_gltf` dependency with `bevy_pbr`"
pull_requests: [22569]
---

Previously, `bevy_gltf` depended on `bevy_pbr`. This meant scene definition was tightly coupled to rendering. This dependency has been inverted, to allow `bevy_gltf` to function without any of the rendering stack present.

You can disable PBR rendering by initializing `PbrPlugin` as so:

```rs
PbrPlugin {
    gltf_render_enabled: false,
    ..Default::default()
}
```

`GltfExtensionHandler` trait's methods have been updated:

- `on_material` passes in the `material_asset : &GltfMaterial` and `material_label: &str`
- `on_spawn_mesh_and_material` also passes in the `material_label: &str`

`UvChannel` has moved from `bevy_pbr` to `bevy_mesh`.
