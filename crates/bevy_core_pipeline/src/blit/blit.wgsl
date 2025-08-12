#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

@group(0) @binding(0) var in_texture: texture_2d<f32>;
@group(0) @binding(1) var in_sampler: sampler;

@fragment
fn fs_main(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    var color = textureSample(in_texture, in_sampler, in.uv);
#ifdef PREMULTIPLY_ALPHA
    color = vec4<f32>(color.rgb * color.a, color.a);
#endif // PREMULTIPLY_ALPHA
    return color;
}
