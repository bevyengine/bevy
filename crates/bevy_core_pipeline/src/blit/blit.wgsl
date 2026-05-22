#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput
#import bevy_core_pipeline::input_texture::{sample_input, current_view_index}
#ifdef SRGB_TO_LINEAR
#import bevy_render::color_operations::srgb_to_linear
#endif
#ifdef OKLAB_TO_LINEAR
#import bevy_render::color_operations::oklab_to_linear_rgb
#endif

@group(0) @binding(1) var in_sampler: sampler;

@fragment
fn fs_main(
    in: FullscreenVertexOutput,
#ifdef MULTIVIEW
    @builtin(view_index) view_index: i32,
#endif
) -> @location(0) vec4<f32> {
#ifdef MULTIVIEW
    current_view_index = view_index;
#endif
    var color = sample_input(in_sampler, in.uv);
#ifdef SRGB_TO_LINEAR
    color = vec4(srgb_to_linear(color.rgb), color.a);
#endif
#ifdef OKLAB_TO_LINEAR
    color = vec4(oklab_to_linear_rgb(color.rgb), color.a);
#endif
    return color;
}
