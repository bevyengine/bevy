// Miscellaneous postprocessing effects, currently just chromatic aberration.

#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput
#import bevy_post_process::effect_stack::chromatic_aberration::chromatic_aberration
#import bevy_post_process::effect_stack::lens_distortion::lens_distortion
#import bevy_post_process::effect_stack::vignette::vignette

@fragment
fn fragment_main(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let color = chromatic_aberration(in.uv);
    let color_distorted = lens_distortion(in.uv, color);
    return vec4(vignette(in.uv, color_distorted), 1.0);
}
