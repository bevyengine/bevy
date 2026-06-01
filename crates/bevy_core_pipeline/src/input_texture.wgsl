#define_import_path bevy_core_pipeline::input_texture

// Shared screen-space input texture for fullscreen post-process pipelines.
//
// Under `#ifdef MULTIVIEW` the binding is a `texture_2d_array<f32>` whose layer
// index selects the view (one layer per eye). Otherwise it is a plain
// `texture_2d<f32>`. Consumers read it through the helpers below where they
// can (no `offset` argument); pipelines that need the const-offset variants
// of `textureSample`/`textureSampleLevel` (e.g. bloom's 13-tap kernel, FXAA's
// neighborhood lumas) must `#ifdef MULTIVIEW` at the callsite — WGSL requires
// the `offset` operand to be a const-expression, so it can't be threaded
// through a helper function parameter.
//
// Per-fragment view index is threaded via `current_view_index` (defaults to 0,
// overwritten from `@builtin(view_index)` at the top of multiview entry-point
// bodies). This mirrors the convention used by `bevy_pbr::mesh_view_bindings`.
#ifdef MULTIVIEW
@group(0) @binding(0) var input_texture: texture_2d_array<f32>;
#else
@group(0) @binding(0) var input_texture: texture_2d<f32>;
#endif

var<private> current_view_index: i32 = 0;

fn sample_input(s: sampler, uv: vec2<f32>) -> vec4<f32> {
#ifdef MULTIVIEW
    return textureSample(input_texture, s, uv, current_view_index);
#else
    return textureSample(input_texture, s, uv);
#endif
}

fn sample_input_level(s: sampler, uv: vec2<f32>, level: f32) -> vec4<f32> {
#ifdef MULTIVIEW
    return textureSampleLevel(input_texture, s, uv, current_view_index, level);
#else
    return textureSampleLevel(input_texture, s, uv, level);
#endif
}
