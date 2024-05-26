#import bevy_render_graph::fullscreen_vertex_shader;

@group(0) @binding(0) src: texture_2d<f32>;
@group(0) @binding(1) smp: sampler;

@fragment
fn blit_frag(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    return textureSample(src, smp, in.uv);
}
