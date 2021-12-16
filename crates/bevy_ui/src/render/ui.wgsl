[[block]]
struct View {
    view_proj: mat4x4<f32>;
    world_position: vec3<f32>;
};
[[group(0), binding(0)]]
var<uniform> view: View;

struct VertexOutput {
    [[location(0)]] uv: vec2<f32>;
    [[location(1)]] color: vec4<f32>;
    [[builtin(position)]] position: vec4<f32>;
};

[[stage(vertex)]]
fn vertex(
    [[location(0)]] vertex_position: vec3<f32>,
    [[location(1)]] vertex_uv: vec2<f32>,
    [[location(2)]] vertex_color: u32,
) -> VertexOutput {
    var out: VertexOutput;
    out.uv = vertex_uv;
    out.position = view.view_proj * vec4<f32>(vertex_position, 1.0);
    out.color = vec4<f32>((vec4<u32>(vertex_color) >> vec4<u32>(0u, 8u, 16u, 24u)) & vec4<u32>(255u)) / 255.0;
    return out;
} 

[[group(1), binding(0)]]
var sprite_texture: texture_2d<f32>;
[[group(1), binding(1)]]
var sprite_sampler: sampler;

[[stage(fragment)]]
fn fragment(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    var color = textureSample(sprite_texture, sprite_sampler, in.uv); 
    color = in.color * color;
    return color;
}