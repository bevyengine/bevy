#import bevy_render::view::View
#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

struct PreviousViewUniforms {
    inverse_view: mat4x4<f32>,
    view_proj: mat4x4<f32>,
}

@group(0) @binding(0) var<uniform> view: View;
@group(0) @binding(1) var<uniform> previous_view: PreviousViewUniforms;

// This vertex shader produces the following, when drawn using indices 0..3:
//
//  1 |  0-----x.....2
//  0 |  |  s  |  . ´
// -1 |  x_____x´
// -2 |  :  .´
// -3 |  1´
//    +---------------
//      -1  0  1  2  3
//
// The axes are clip-space x and y. The region marked s is the visible region.
// The digits in the corners of the right-angled triangle are the vertex
// indices.
//
// The top-left has UV 0,0, the bottom-left has 0,2, and the top-right has 2,0.
// This means that the UV gets interpolated to 1,1 at the bottom-right corner
// of the clip-space rectangle that is at 1,-1 in clip space.
@vertex
fn vertex_main(@builtin(vertex_index) vertex_index: u32) -> FullscreenVertexOutput {
    // See the explanation above for how this works
    let uv = vec2<f32>(f32(vertex_index >> 1u), f32(vertex_index & 1u)) * 2.0;
    let clip_position = vec4<f32>(uv * vec2<f32>(2.0, -2.0) + vec2<f32>(-1.0, 1.0), 0.0, 1.0);

    return FullscreenVertexOutput(clip_position, uv);
}

@fragment
fn fragment_main(in: FullscreenVertexOutput) -> @location(1) vec4<f32> {
    let clip_pos = in.uv * vec2(2.0, -2.0) + vec2(-1.0, 1.0);
    let world_pos = view.inverse_view_proj * vec4(clip_pos, 0.001, 1.0);
    let prev_clip_pos = (previous_view.view_proj * world_pos).xy;
    let velocity = (clip_pos - prev_clip_pos) * vec2(0.5, -0.5);
    return vec4(velocity.x, velocity.y, 0.0, 1.0);
}
