---
title: "Invert `bevy_gltf` dependency with `bevy_pbr`"
pull_requests: [22569]
---

Previously, `bevy_gltf` depends on `bevy_pbr` , with a tight coupling between the scene definition and the rendering of the scene. This has been inverted.

You can disable PBR rendering by initialising `PbrPlugin` as so:

```rs
PbrPlugin {
    gltf_render_enabled: false,
    ..Default::default()
}
```

`GltfExtensionHandler` trait's methods have been updated:

- `on_material` passes in the `material_asset : &GltfMaterial` and `material_label: &str`
- `on_spawn_mesh_and_material` also passes in the `material_label: &str`
