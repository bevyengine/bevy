#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput
#import bevy_core_pipeline::input_texture::in_texture

@group(0) @binding(1) var in_sampler: sampler;

@fragment
fn fs_main(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    return textureSample(
        in_texture, 
        in_sampler, 
        in.uv,
#ifdef MULTIVIEW
        in.view_index
#endif
    );
}
