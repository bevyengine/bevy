struct View {
    view_proj: mat4x4<f32>;
    world_position: vec3<f32>;
};
[[group(0), binding(0)]]
var<uniform> view: View;

struct VertexOutput {
    [[location(0)]] uv: vec2<f32>;
    [[location(1)]] color: vec4<f32>;
    [[location(2)]] center: vec2<f32>;
    [[location(3)]] uv_min: vec2<f32>;
    [[location(4)]] uv_max: vec2<f32>;
    [[location(5)]] size: vec2<f32>;
    [[location(6)]] border_color: vec4<f32>;
    [[location(7)]] border_width: f32;
    [[location(8)]] border_radius: vec4<f32>;
    [[builtin(position)]] position: vec4<f32>;
};

[[stage(vertex)]]
fn vertex(
    [[location(0)]] vertex_position: vec3<f32>,
    [[location(1)]] vertex_uv: vec2<f32>,
    [[location(2)]] vertex_color: u32,
    [[location(3)]] uv_min: vec2<f32>,
    [[location(4)]] uv_max: vec2<f32>,
    [[location(5)]] size: vec2<f32>,
    [[location(6)]] border_color: u32,
    [[location(7)]] border_width: f32,
    [[location(8)]] border_radius: vec4<f32>,
) -> VertexOutput {
    var out: VertexOutput;
    out.uv = vertex_uv;
    out.position = view.view_proj * vec4<f32>(vertex_position, 1.0);
    out.color = vec4<f32>((vec4<u32>(vertex_color) >> vec4<u32>(0u, 8u, 16u, 24u)) & vec4<u32>(255u)) / 255.0;
    out.size = size;
    out.uv_min = uv_min;
    out.uv_max = uv_max;
    out.border_width = border_width;
    out.border_color = vec4<f32>((vec4<u32>(border_color) >> vec4<u32>(0u, 8u, 16u, 24u)) & vec4<u32>(255u)) / 255.0;
    out.border_radius = border_radius;
    return out;
}

[[group(1), binding(0)]]
var sprite_texture: texture_2d<f32>;
[[group(1), binding(1)]]
var sprite_sampler: sampler;

fn distance_round_border(point: vec2<f32>, size: vec2<f32>, radius_by_corner: vec4<f32>) -> f32 {
    var corner_index = select(0, 1, point.y > 0.0) + select(0, 2, point.x > 0.0);
    var radius = radius_by_corner[corner_index];
    var q = abs(point) - size + radius;
    return min(max(q.x, q.y), 0.0) + length(max(q, vec2<f32>(0.0))) - radius;
}

[[stage(fragment)]]
fn fragment(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    var color = textureSample(sprite_texture, sprite_sampler, in.uv);
    color = in.color * color;

    // this makes rounded borders look softer, but it's affecting colors so I'm excluding it for now
    var edge_softness = 0.0; //clamp(in.border_width - 1.0, 0.0, 2.0);
    var border_softness = 0.0; //clamp(in.border_width - 1.0, 0.0, 1.0);

    // clamp radius between (0.0) and (shortest side / 2.0)
    var radius = clamp(in.border_radius, vec4<f32>(0.0), vec4<f32>(min(in.size.x, in.size.y) / 2.0));
    
    // get a normalized point based on uv, uv_max and uv_min
    var point = ((in.uv - in.uv_min) / (in.uv_max - in.uv_min) - vec2<f32>(0.5)) * in.size;
    var distance = distance_round_border(point, in.size * 0.5, radius);

    var inner_alpha = 1.0 - smoothStep(0.0, edge_softness, distance + edge_softness);
    var border_alpha = 1.0 - smoothStep(in.border_width - border_softness, in.border_width, abs(distance));
    color = mix(vec4<f32>(0.0), mix(color, in.border_color, border_alpha), inner_alpha);

    return color;
}
