#import bevy_render::view

@group(0) @binding(0)
var<uniform> view: View;

struct VertexOutput {
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) size: vec2<f32>,
    @location(3) point: vec2<f32>,
    @location(4) border_color: vec4<f32>,
    @location(5) border_width: f32,
    @location(6) radius: f32,
    @location(7)  pos: vec2<f32>,
    @builtin(position) position: vec4<f32>,
 
};

struct UiUniformEntry {
    color: u32,
    size: vec2<f32>,
    center: vec2<f32>,
    border_color: u32,
    border_width: f32,
    corner_radius: vec4<f32>,
};

struct UiUniform {
    // NOTE: this array size must be kept in sync with the constants defined bevy_ui/src/render/mod.rs
    entries: array<UiUniformEntry, 256u>,
};

@group(2) @binding(0)
var<uniform> ui_uniform: UiUniform;


fn unpack_color_from_u32(color: u32) -> vec4<f32> {
    return vec4<f32>((vec4<u32>(color) >> vec4<u32>(0u, 8u, 16u, 24u)) & vec4<u32>(255u)) / 255.0;
}


@vertex
fn vertex(
    @location(0) vertex_position: vec3<f32>,
    @location(1) vertex_uv: vec2<f32>,
    @location(2) vertex_color: vec4<f32>,
    @location(3) ui_uniform_index: u32,
) -> VertexOutput {
    var out: VertexOutput;
    var node = ui_uniform.entries[ui_uniform_index];

    out.size = node.size;
    out.point = vertex_position.xy - node.center;
    out.border_width = node.border_width;
    out.border_color = unpack_color_from_u32(node.border_color);

    // get radius for this specific corner
    var corner_index = select(0, 1, out.position.y > 0.0) + select(0, 2, out.position.x > 0.0);
    out.radius = node.corner_radius[corner_index];

    // clamp radius between (0.0) and (shortest side / 2.0)
    out.radius = clamp(out.radius, 0.0, min(out.size.x, out.size.y) / 2.0);

    out.pos = vertex_position.xy;
    out.uv = vertex_uv;
    out.position = view.view_proj * vec4<f32>(vertex_position, 1.0);
    out.color = unpack_color_from_u32(node.color);
    return out;
}


fn distance_alg(
    frag_coord: vec2<f32>,
    position: vec2<f32>,
    size: vec2<f32>,
    radius: f32
) -> f32 {
    var inner_size: vec2<f32> = size - vec2<f32>(radius, radius) * 2.0;
    var top_left: vec2<f32> = position + vec2<f32>(radius, radius);
    var bottom_right: vec2<f32> = top_left + inner_size;

    var top_left_distance: vec2<f32> = top_left - frag_coord;
    var bottom_right_distance: vec2<f32> = frag_coord - bottom_right;

    var dist: vec2<f32> = vec2<f32>(
        max(max(top_left_distance.x, bottom_right_distance.x), 0.0),
        max(max(top_left_distance.y, bottom_right_distance.y), 0.0)
    );

    return sqrt(dist.x * dist.x + dist.y * dist.y);
}

// Based on the fragement position and the center of the quad, select one of the 4 radi.
// Order matches CSS border radius attribute:
// radi.x = top-left, radi.y = top-right, radi.z = bottom-right, radi.w = bottom-left
fn select_border_radius(radi: vec4<f32>, position: vec2<f32>, center: vec2<f32>) -> f32 {
    var rx = radi.x;
    var ry = radi.y;
    rx = select(radi.x, radi.y, position.x > center.x);
    ry = select(radi.w, radi.z, position.x > center.x);
    rx = select(rx, ry, position.y > center.y);
    return rx;
}


@group(1) @binding(0)
var sprite_texture: texture_2d<f32>;
@group(1) @binding(1)
var sprite_sampler: sampler;

fn distance_round_border(point: vec2<f32>, size: vec2<f32>, radius: f32) -> f32 {
    var q = abs(point) - size + radius;
    return min(max(q.x, q.y), 0.0) + length(max(q, vec2<f32>(0.0))) - radius;
}
@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    var color = textureSample(sprite_texture, sprite_sampler, in.uv);
    color = in.color * color;
    


    var border_radius = in.radius;
    var scale = vec2(1.0, 1.0);

    if (in.border_width > 0.0) {
        var internal_border: f32 = max(border_radius - in.border_width, 0.0);

        var internal_distance: f32 = distance_alg(
            in.position.xy,
            in.pos + vec2<f32>(in.border_width, in.border_width),
            scale - vec2<f32>(in.border_width * 2.0, in.border_width * 2.0),
            internal_border
        );

        var border_mix: f32 = smoothstep(
            max(internal_border - 0.5, 0.0),
            internal_border + 0.5,
            internal_distance
        );

        color = mix(in.color, in.border_color, vec4<f32>(border_mix, border_mix, border_mix, border_mix));
    }

    var dist: f32 = distance_alg(
        vec2<f32>(in.position.x, in.position.y),
        in.pos,
        scale,
        border_radius
    );

    var radius_alpha: f32 = 1.0 - smoothstep(
        max(border_radius - 0.5, 0.0),
        border_radius + 0.5,
        dist
    );

    return vec4<f32>(color.x, color.y, color.z, color.w * radius_alpha);
}
