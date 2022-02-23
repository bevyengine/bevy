struct View {
    view_proj: mat4x4<f32>;
    world_position: vec3<f32>;
};
[[group(0), binding(0)]]
var<uniform> view: View;

struct UiUniformEntry {
    color: u32;
    size: vec2<f32>;
    center: vec2<f32>;
    border_color: u32;
    border_width: f32;
    corner_radius: vec4<f32>;
};

struct UiUniform {
    // NOTE: this array size must be kept in sync with the constants defined bevy_ui/src/render/mod.rs
    entries: array<UiUniformEntry, 256u>;
};

[[group(2), binding(0)]]
var<uniform> ui_uniform: UiUniform;

struct VertexOutput {
    [[location(0)]] uv: vec2<f32>;
    [[location(1)]] color: vec4<f32>;
    [[location(2)]] size: vec2<f32>;
    [[location(3)]] point: vec2<f32>;
    [[location(4)]] border_color: vec4<f32>;
    [[location(5)]] border_width: f32;
    [[location(6)]] radius: f32;
    [[builtin(position)]] position: vec4<f32>;
};

fn unpack_color_from_u32(color: u32) -> vec4<f32> {
    return vec4<f32>((vec4<u32>(color) >> vec4<u32>(0u, 8u, 16u, 24u)) & vec4<u32>(255u)) / 255.0;
}

[[stage(vertex)]]
fn vertex(
    [[location(0)]] vertex_position: vec3<f32>,
    [[location(1)]] vertex_uv: vec2<f32>,
    [[location(2)]] ui_uniform_index: u32,
) -> VertexOutput {
    var out: VertexOutput;
    var node = ui_uniform.entries[ui_uniform_index];
    out.uv = vertex_uv;
    out.position = view.view_proj * vec4<f32>(vertex_position, 1.0);
    out.color = unpack_color_from_u32(node.color);
    out.size = node.size;
    out.point = vertex_position.xy - node.center;
    out.border_width = node.border_width;
    out.border_color = unpack_color_from_u32(node.border_color);
    
    var corner_index = select(0, 1, out.position.y > 0.0) + select(0, 2, out.position.x > 0.0);
    out.radius = node.corner_radius[corner_index];

    return out;
}

[[group(1), binding(0)]]
var sprite_texture: texture_2d<f32>;
[[group(1), binding(1)]]
var sprite_sampler: sampler;

fn distance_round_border(point: vec2<f32>, size: vec2<f32>, radius: f32) -> f32 {
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
    var radius = clamp(in.radius, 0.0, min(in.size.x, in.size.y) / 2.0);
    
    var distance = distance_round_border(in.point, in.size * 0.5, radius);

    var inner_alpha = 1.0 - smoothStep(0.0, edge_softness, distance + edge_softness);
    var border_alpha = 1.0 - smoothStep(in.border_width - border_softness, in.border_width, abs(distance));
    color = mix(vec4<f32>(0.0), mix(color, in.border_color, border_alpha), inner_alpha);

    return color;
}
