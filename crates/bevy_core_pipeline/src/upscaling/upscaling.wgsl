struct VertexOutput {
    [[builtin(position)]]
    position: vec4<f32>;
    [[location(0)]]
    uv: vec2<f32>;
};

[[stage(vertex)]]
fn vs_main([[builtin(vertex_index)]] in_vertex_index: u32) -> VertexOutput {
    let uv = vec2<f32>(f32((in_vertex_index << 1u) & 2u), f32(in_vertex_index & 2u));
    let position = vec4<f32>(uv * vec2<f32>(2.0, -2.0) + vec2<f32>(-1.0, 1.0), 0.0, 1.0);
    return VertexOutput(position, uv);
}

[[group(0), binding(0)]]
var hdr_texture: texture_2d<f32>;
[[group(0), binding(1)]]
var hdr_sampler: sampler;

[[stage(fragment)]]
fn fs_main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    let hdr_color = textureSample(hdr_texture, hdr_sampler, in.uv);

    return hdr_color;
}
