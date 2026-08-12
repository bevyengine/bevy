import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput;
import bevy_core_pipeline::input_texture::{sample_input, current_view_index};
@if(SRGB_TO_LINEAR) import bevy_render::color_operations::srgb_to_linear;
@if(OKLAB_TO_LINEAR) import bevy_render::color_operations::oklab_to_linear_rgb;

@group(0) @binding(1) var in_sampler: sampler;

@fragment
fn fs_main(
    in: FullscreenVertexOutput,
@if(MULTIVIEW)
    @builtin(view_index) view_index: u32,
) -> @location(0) vec4<f32> {
@if(MULTIVIEW)
    current_view_index = view_index;
    var color = sample_input(in_sampler, in.uv);
    @if(SRGB_TO_LINEAR)
    color = vec4(srgb_to_linear(color.rgb), color.a);
    @if(OKLAB_TO_LINEAR)
    color = vec4(oklab_to_linear_rgb(color.rgb), color.a);
    return color;
}
