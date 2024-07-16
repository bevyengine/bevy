// Miscellaneous postprocessing effects, currently just chromatic aberration.

#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput
#import bevy_core_pipeline::post_processing::chromatic_aberration::chromatic_aberration

@fragment
fn fragment_main(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    return vec4(chromatic_aberration(in.uv), 1.0);
}
