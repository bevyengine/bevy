// Miscellaneous postprocessing effects.

#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput
#import bevy_post_process::effect_stack::chromatic_aberration::chromatic_aberration
#import bevy_post_process::effect_stack::film_grain::film_grain
#import bevy_post_process::effect_stack::vignette::vignette

@fragment
fn fragment_main(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let ca_color = chromatic_aberration(in.uv);
    let v_color = vignette(in.uv, ca_color);
    return vec4(film_grain(in.uv, v_color), 1.0);
}
