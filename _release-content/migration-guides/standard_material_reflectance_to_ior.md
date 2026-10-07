---
title: "`StandardMaterial::reflectance` is replaced by `ior` and `specular`"
pull_requests: [24552]
---

`StandardMaterial::reflectance` and `GltfMaterial::reflectance` have been removed.
The specular reflectance of non-metals now comes from `StandardMaterial::ior`, and the new `specular` field is a linear weight that scales the whole specular response of non-metals.
`specular` defaults to `1.0`, and `ior` defaults to `1.5`, which gives the same 4% reflectance as the old default `reflectance` of `0.5`.

Previously, `reflectance` mapped to the reflectance at normal incidence (F0) as `0.16 * reflectance * reflectance`.
To keep the same F0 for an old `reflectance` value `r`, set `ior` to `(1.0 + 0.4 * r) / (1.0 - 0.4 * r)`:

```rust
// 0.20
StandardMaterial {
    reflectance: 0.35,
    ..default()
}

// 0.21
StandardMaterial {
    ior: 1.33, // (1.0 + 0.4 * 0.35) / (1.0 - 0.4 * 0.35)
    ..default()
}
```

`ior: 0.0` gives an F0 of 1.0, which is what an old `reflectance` of `2.5` gave.

If you set a low `reflectance` to remove reflections, set `specular` to `0.0` instead.

`specular_tint` now scales F0 linearly.
Previously, it multiplied `reflectance` before the squaring, so F0 was `0.16 * (specular_tint * reflectance)^2`.
To keep the same F0 with a constant tint other than white, square each linear RGB channel of the tint.
For example, a tint of `Color::linear_rgb(0.5, 0.5, 0.5)` becomes `Color::linear_rgb(0.25, 0.25, 0.25)`.

`specular_texture` and `specular_tint_texture` now scale `specular` and `specular_tint` linearly, as `KHR_materials_specular` defines.
Previously, they scaled `reflectance`, so their effect on F0 was squared.
`specular_texture` also had an extra `0.5` factor, so a white specular texture with the default `reflectance` of `0.5` gave an F0 of 1% instead of 4%.
For a material with a `specular_texture`, apply the IOR formula above to `0.5 * reflectance` instead of `reflectance`.
For example, a `reflectance` of `2.0` with a specular texture becomes an `ior` of `2.33`.

The reflectance at grazing angles (F90) is now always 1.0, scaled by `specular`.
Previously, F90 dropped below 1.0 when F0 was below about 2%, so a low `reflectance` also dimmed reflections at grazing angles.

In shaders, the `reflectance` field of the `StandardMaterial` uniform and of `PbrInput::material` is replaced by `specular_tint` and `specular_weight`.
`calculate_F0` and `pbr_lighting::fresnel` are removed, and `calculate_F0_dielectric` takes the IOR and the specular tint.
`LightingInput` has a new `specular_weight` field.
`ambient_light` takes a `specular_F90` argument after `specular_color`, and `specular_transmissive_light` takes an `F90` argument after `F0`.
`pbr_lighting::specular_F0` and `pbr_lighting::specular_F90` compute these values from the dielectric F0, the metallic F0, the specular weight and the metallic factor.

The deferred renderer now supports `specular_tint` and `specular_tint_texture` when the device's color attachment limits have room for one more G-buffer target and it allows more than 16 sampled textures per shader stage; otherwise, as on WebGL2, the tint is white.
It stores the tint in a new G-buffer target, `ViewPrepassTextures::deferred_specular_tint`, with the format `DEFERRED_SPECULAR_TINT_FORMAT` (`R32Uint`).
The deferred pass writes it at `@location(4)`, after the lighting pass ID, and deferred prepass shaders get the `DEFERRED_SPECULAR_TINT` shader def when the target exists.
`prepass_target_descriptors` takes a new `deferred_specular_tint` argument; pass the result of `deferred_specular_tint_fits`.
If you build the color targets of a deferred pass pipeline yourself, add a target with this format when it fits.
If you build `ViewPrepassTextures` yourself, add the new field.

If your custom deferred shader uses `bevy_pbr::prepass::io::FragmentOutput` and `deferred_output`, you don't need to change anything.
If it declares its own fragment output struct, add the new target:

```wgsl
// 0.20
struct FragmentOutput {
    @location(2) deferred: vec4<u32>,
    @location(3) deferred_lighting_pass_id: u32,
}

// 0.21
struct FragmentOutput {
    @location(2) deferred: vec4<u32>,
    @location(3) deferred_lighting_pass_id: u32,
    @if(DEFERRED_SPECULAR_TINT)
    @location(4) deferred_specular_tint: u32,
}
```

If it fills `deferred` with `deferred_gbuffer_from_pbr_input`, also fill the tint with `bevy_pbr::deferred::types::pack_specular_tint`:

```wgsl
// 0.20
out.deferred = deferred_gbuffer_from_pbr_input(pbr_input);

// 0.21
out.deferred = deferred_gbuffer_from_pbr_input(pbr_input);
@if(DEFERRED_SPECULAR_TINT)
out.deferred_specular_tint = pack_specular_tint(pbr_input.material.specular_tint);
```

A packed value of `0` is a white tint, so leaving the field unwritten gives the old behavior.
Render passes that write the G-buffer outside the deferred prepass don't need the new target; their pixels keep the tint that the deferred prepass wrote, which is white where it wrote nothing.

On the reading side, `pbr_input_from_deferred_gbuffer` loads the tint from the new `deferred_specular_tint_texture` mesh view binding at `frag_coord.xy` when the `DEFERRED_SPECULAR_TINT` shader def is set, and uses white otherwise.
Custom deferred lighting pipelines get the tint by setting that def when `MeshPipelineViewLayouts::has_deferred_specular_tint` returns `true` for their view layout key.
`prepass::get_bind_group_layout_entries` and `prepass::get_bindings` now return 5 entries, and the fifth one goes to binding 39 of the mesh view bind group.
`prepass::get_bind_group_layout_entries` takes a new `deferred_specular_tint` argument, which says whether the view has a tint texture.
