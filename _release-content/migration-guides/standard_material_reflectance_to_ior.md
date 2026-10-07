---
title: "`StandardMaterial::reflectance` is replaced by `ior` and `specular`"
pull_requests: [24552]
---

`StandardMaterial::reflectance` and `GltfMaterial::reflectance` have been removed.
The reflectance of non-metals now comes from `ior`, and the new `specular` field scales it.
The defaults (`ior: 1.5`, `specular: 1.0`) look the same as the old default `reflectance` of `0.5`.

To keep the look of an old `reflectance` value `r`, set `ior` to `(1.0 + 0.4 * r) / (1.0 - 0.4 * r)`:

```rust
// 0.20
StandardMaterial {
    reflectance: 0.35,
    ..default()
}

// 0.21
StandardMaterial {
    ior: 1.33,
    ..default()
}
```

If you set a low `reflectance` to remove reflections, set `specular: 0.0` instead.

`specular_tint`, `specular_texture` and `specular_tint_texture` now scale reflectance linearly.
To keep the look of a non-white `specular_tint`, square each of its linear channels.
If a material has a `specular_texture`, use `0.5 * reflectance` in the IOR formula above.

In shaders:

- `PbrInput::material.reflectance` is replaced by `specular_tint` and `specular_weight`.
- `calculate_F0` and `pbr_lighting::fresnel` are removed. `calculate_F0_dielectric` takes the IOR and the specular tint.
- `LightingInput` has a new `specular_weight` field.
- `ambient_light` takes `specular_F90` after `specular_color`, and `specular_transmissive_light` takes `F90` after `F0`.
- `environment_map_light` takes a `LobeMultiscatter` after `input`. Build it with `compute_lobe_multiscatter`.

If your deferred prepass shader declares its own fragment output struct, add the specular tint target:

```wgsl
// 0.21
struct FragmentOutput {
    @location(2) deferred: vec4<u32>,
    @location(3) deferred_lighting_pass_id: u32,
    @if(DEFERRED_SPECULAR_TINT)
    @location(4) deferred_specular_tint: u32,
}
```

Write it with `pack_specular_tint`, or leave it at `0`, which is a white tint.

If you build deferred prepass pipelines or textures yourself:

- `prepass_target_descriptors` takes a `deferred_specular_tint` argument. Pass `DeferredSpecularTintSupport::is_supported`.
- `ViewPrepassTextures` has a new `deferred_specular_tint` field.
- `prepass::get_bindings` returns five texture views.
