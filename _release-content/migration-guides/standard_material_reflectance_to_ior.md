---
title: "`StandardMaterial::reflectance` is replaced by `ior` and `specular_weight`"
pull_requests: [26175]
---

`StandardMaterial::reflectance` and `GltfMaterial::reflectance` have been removed.
The reflectance of non-metals now comes from `ior`, and the new `specular_weight` field scales it, as in OpenPBR.
The defaults (`ior: 1.5`, `specular_weight: 1.0`) look the same as the old default `reflectance` of `0.5`.

To keep the look of an old `reflectance` value `r`, set `specular_weight` to `4.0 * r * r`:

```rust
// 0.20
StandardMaterial {
    reflectance: 1.0,
    ..default()
}

// 0.21
StandardMaterial {
    specular_weight: 4.0,
    ..default()
}
```

`specular_tint`, `specular_tint_texture` and the alpha of `specular_texture` now scale reflectance linearly, so square their linear values to keep the old look.
`specular_tint` is clamped to `[0.0, 1.0]`: move any brightening into `specular_weight`.
With a `specular_texture`, use `r * r` for `specular_weight`.

Under a clearcoat, `ior` is now relative to the clearcoat's IOR of 1.5, so an `ior` of `1.5` doesn't reflect under a full clearcoat.
To keep the old look with a full clearcoat, multiply `ior` by 1.5.

In shaders:

- `PbrInput::material.reflectance` is replaced by `specular_tint` and `specular_weight`.
- `calculate_F0`, `calculate_F0_dielectric` and `pbr_lighting::fresnel` are removed. Use `calculate_dielectric_reflectance`.
- `LightingInput::F0_dielectric` is an `f32`, next to new `F90_dielectric` and `specular_tint` fields.
- `ambient_light` takes `specular_F90` after `specular_color`. `specular_transmissive_light` takes the dielectric `F0` and `F90` as `f32`s.
- `environment_map_light` takes a `LobeMultiscatter` from `compute_lobe_multiscatter` after `input`.
- The G-buffer stores `encode_fresnel_ratio` where it stored reflectance, and `pbr_input_from_deferred_gbuffer` returns an `ior` and `specular_weight` that reproduce it, not the material's.

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
