@group(0) @binding(0) var accum_texture: texture_2d<f32>;
@group(0) @binding(1) var reveal_texture: texture_2d<f32>;
@group(0) @binding(2) var samp: sampler;

struct FullscreenVertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let accum = textureSample(accum_texture, samp, in.uv);
    let reveal = textureSample(reveal_texture, samp, in.uv).r;

    return vec4f(accum.rgb / max(accum.a, 1e-5), reveal);
}
