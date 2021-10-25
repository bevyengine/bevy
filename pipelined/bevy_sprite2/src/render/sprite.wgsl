[[block]]
struct View {
    view_proj: mat4x4<f32>;
    world_position: vec3<f32>;
};
[[group(0), binding(0)]]
var<uniform> view: View;

struct VertexOutput {
    [[location(0)]] uv: vec2<f32>;
    [[builtin(position)]] position: vec4<f32>;
};

[[stage(vertex)]]
fn vertex(
    [[location(0)]] vertex_position: vec3<f32>,
    [[location(1)]] vertex_uv: vec2<f32>
) -> VertexOutput {
    var out: VertexOutput;
    out.uv = vertex_uv;
    out.position = view.view_proj * vec4<f32>(vertex_position, 1.0);
    return out;
} 

[[group(1), binding(0)]]
var sprite_texture: texture_2d<f32>;
[[group(1), binding(1)]]
var sprite_sampler: sampler;

[[block]]
struct SpriteUniforms {
    size: vec2<f32>;
    uv_min: vec2<f32>;
    uv_max: vec2<f32>;
    border_radius: f32;
};
[[group(2), binding(0)]]
var<uniform> sprite_uniforms: SpriteUniforms;

// Calculate the distance from the fragment to the border of the rounded rectangle,
// return negative value when the fragment is inside the rounded rectangle.
fn distance_round_border(point: vec2<f32>, size: vec2<f32>, radius: f32) -> f32 {
    let dr = abs(point) - (size - radius);
    let d = length(max(dr, vec2<f32>(0.0))) - radius;
    let t = min(dr, vec2<f32>(0.0));
    let d_extra = max(t.x, t.y);

    return d + d_extra;
}

[[stage(fragment)]]
fn fragment(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    var color = textureSample(sprite_texture, sprite_sampler, in.uv);

    if (sprite_uniforms.border_radius > 0.0) {
        let d = distance_round_border(
            ((in.uv - sprite_uniforms.uv_min) / (sprite_uniforms.uv_max - sprite_uniforms.uv_min) - vec2<f32>(0.5)) * sprite_uniforms.size, 
            sprite_uniforms.size * 0.5, 
            sprite_uniforms.border_radius
        );
        let softness = 0.33;
        color.a = color.a * (1.0 - smoothStep(-softness, softness, d));
    }

    return color;
}