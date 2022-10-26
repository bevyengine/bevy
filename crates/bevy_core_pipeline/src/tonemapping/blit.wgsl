#import bevy_core_pipeline::fullscreen_vertex_shader

@group(0) @binding(0)
var texture: texture_2d<f32>;
@group(0) @binding(1)
var texture_sampler: sampler;

@fragment
fn fs_main(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    return textureSample(texture, texture_sampler, in.uv);
}
