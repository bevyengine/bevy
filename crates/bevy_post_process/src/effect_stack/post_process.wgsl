// Miscellaneous postprocessing effects, currently just chromatic aberration.

#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput
#import bevy_post_process::effect_stack::chromatic_aberration::chromatic_aberration
#import bevy_post_process::effect_stack::lens_distortion::lens_distortion
#import bevy_post_process::effect_stack::vignette::vignette

@fragment
fn fragment_main(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let distorted_uv = lens_distortion(in.uv);
    let color = chromatic_aberration(distorted_uv);
    return vec4(vignette(in.uv, color), 1.0);
}
