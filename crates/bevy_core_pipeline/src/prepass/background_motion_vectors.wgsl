#import bevy_render::view::View
#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput
#import bevy_pbr::view_transformations::uv_to_ndc

struct PreviousViewUniforms {
    view_from_world: mat4x4<f32>,
    clip_from_world: mat4x4<f32>,
    clip_from_view: mat4x4<f32>,
    world_from_clip: mat4x4<f32>,
    view_from_clip: mat4x4<f32>,
}

// View uniform, shaped the same as `bevy_pbr::mesh_view_bindings::view_array`
// so the background motion vectors pipeline can share the packed
// `DynamicArrayUniformBuffer` behind `ViewUniforms`. The shader reads through
// `view()`, which indexes the array at `current_view_index` (set from
// `@builtin(view_index)` at the top of the fragment under MULTIVIEW). For
// non-multiview pipelines, `MAX_VIEW_COUNT` is undefined and the fallback
// `array<View, 1>` matches the single `ViewUniform` packed per camera.
#ifdef MAX_VIEW_COUNT
@group(0) @binding(0) var<uniform> view_array: array<View, #{MAX_VIEW_COUNT}>;
#else
@group(0) @binding(0) var<uniform> view_array: array<View, 1>;
#endif
var<private> current_view_index: i32 = 0;

fn view() -> View {
    return view_array[current_view_index];
}

@group(0) @binding(1) var<uniform> previous_view: PreviousViewUniforms;

/// Writes motion vectors for sky pixels (depth == 0 in reversed-Z) based on camera rotation.
///
/// The fullscreen vertex outputs z=0.0, which equals the cleared depth at sky pixels in
/// reversed-Z. The GreaterEqual depth test passes only where depth == 0, so this only writes
/// to pixels untouched by geometry.
@fragment
fn fragment(
    in: FullscreenVertexOutput,
#ifdef MULTIVIEW
    @builtin(view_index) view_index: i32,
#endif
) -> @location(1) vec4<f32> {
#ifdef MULTIVIEW
    current_view_index = view_index;
#endif
    let clip_pos = uv_to_ndc(in.uv);
    let world_pos = view().world_from_clip * vec4(clip_pos, 0.0, 1.0);
    // Use unjittered_clip_from_world for the current frame to strip TAA jitter from the
    // motion vector.
    let curr_clip_pos = (view().unjittered_clip_from_world * world_pos).xy;
    let prev_clip_pos = (previous_view.clip_from_world * world_pos).xy;
    let velocity = (curr_clip_pos - prev_clip_pos) * vec2(0.5, -0.5);
    return vec4(velocity.x, velocity.y, 0.0, 1.0);
}
