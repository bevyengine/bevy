#import bevy_core_pipeline::fullscreen_vertex_shader

@group(0) @binding(0)
var hdr_texture: texture_2d<f32>;
@group(0) @binding(1)
var hdr_sampler: sampler;

@fragment
fn fs_main(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let hdr_color = textureSample(hdr_texture, hdr_sampler, in.uv);

    return hdr_color;
}
