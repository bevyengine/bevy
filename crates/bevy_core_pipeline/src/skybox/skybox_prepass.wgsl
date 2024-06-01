#import bevy_render::view::View
#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

struct PreviousViewUniforms {
    inverse_view: mat4x4<f32>,
    view_proj: mat4x4<f32>,
}

@group(0) @binding(0) var<uniform> view: View;
@group(0) @binding(1) var<uniform> previous_view: PreviousViewUniforms;

@fragment
fn fragment_main(in: FullscreenVertexOutput) -> @location(1) vec4<f32> {
    let clip_pos = in.uv * vec2(2.0, -2.0) + vec2(-1.0, 1.0); // Convert from uv to clip space
    let world_pos = view.inverse_view_proj * vec4(clip_pos, 0.0, 1.0);
    let prev_clip_pos = (previous_view.view_proj * world_pos).xy;
    let velocity = (clip_pos - prev_clip_pos) * vec2(0.5, -0.5); // Copied from mesh motion vectors

    return vec4(velocity.x, velocity.y, 0.0, 1.0);
}
