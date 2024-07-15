#import bevy_render::view::View
#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput
#import bevy_pbr::view_transformations::uv_to_ndc

struct PreviousViewUniforms {
    view_from_world: mat4x4<f32>,
    clip_from_world: mat4x4<f32>,
}

@group(0) @binding(0) var<uniform> view: View;
@group(0) @binding(1) var<uniform> previous_view: PreviousViewUniforms;

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(1) vec4<f32> {
    let clip_pos = uv_to_ndc(in.uv); // Convert from uv to clip space
    let world_pos = view.world_from_clip * vec4(clip_pos, 0.0, 1.0);
    let prev_clip_pos = (previous_view.clip_from_world * world_pos).xy;
    let velocity = (clip_pos - prev_clip_pos) * vec2(0.5, -0.5); // Copied from mesh motion vectors

    return vec4(velocity.x, velocity.y, 0.0, 1.0);
}
