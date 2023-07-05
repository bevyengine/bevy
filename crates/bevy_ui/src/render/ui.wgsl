#import bevy_render::view  View

const TEXTURED_QUAD = 1u;
const BOX_SHADOW = 2u;
const DISABLE_AA = 4u;
const RIGHT_VERTEX = 8u;
const BOTTOM_VERTEX = 16u;

@group(0) @binding(0)
var<uniform> view: View;

struct VertexOutput {
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) @interpolate(flat) size: vec2<f32>,
    @location(3) @interpolate(flat) flags: u32,
    @location(4) @interpolate(flat) radius: vec4<f32>,    
    @location(5) @interpolate(flat) thickness: vec4<f32>,    
    @builtin(position) position: vec4<f32>,
};

@vertex
fn vertex(
    @location(0) vertex_position: vec3<f32>,
    @location(1) vertex_uv: vec2<f32>,
    @location(2) vertex_color: vec4<f32>,
    @location(3) flags: u32,
    // x = top right, y = bottom right, z = bottom left, w = top left
    @location(4) radius: vec4<f32>,
    @location(5) thickness: vec4<f32>,
    @location(6) size: vec2<f32>,
    @location(7) border_color: vec4<f32>,
) -> VertexOutput {
    var out: VertexOutput;
    out.uv = vertex_uv;
    out.position = view.view_proj * vec4<f32>(vertex_position, 1.0);
    out.color = vertex_color;
    out.border_color = border_color;
    out.flags = flags;
    out.radius = radius;
    out.size = size;
    out.thickness = thickness;
    return out;
}

@group(1) @binding(0)
var sprite_texture: texture_2d<f32>;

@group(1) @binding(1)
var sprite_sampler: sampler;

fn sigmoid(t: f32) -> f32 {
    return 1.0 / (1.0 + exp(-t));
}

fn sd_rounded_box(point: vec2<f32>, size: vec2<f32>, radius: vec4<f32>) -> f32 {
    let top_right_radius = radius.x;
    let bottom_right_radius = radius.y;
    let bottom_left_radius = radius.z;
    let top_left_radius = radius.w;
    var r: f32 = top_left_radius;
    if 0.0 < point.x {
        if 0.0 < point.y {
            r = bottom_right_radius;
        } else {
            r = top_right_radius;
        }
    } else {
        if 0.0 < point.y {
            r = bottom_left_radius;
        } else {
            r = top_left_radius;
        }
    }
    let q = abs(point) - 0.5 * size + r;
    return length(max(q, vec2(0.0, 0.0))) + min(max(q.x, q.y), 0.0) - r;
}

fn sd_inset_rounded_box(point: vec2<f32>, size: vec2<f32>, radius: vec4<f32>, inset: vec4<f32>) -> f32 {
    let inner_size = size - inset.xy - inset.zw;
    let inner_center = inset.xy + 0.5 * inner_size - 0.5 *size;
    let inner_point = point - inner_center;

    var r = radius;

    // top right corner 
    r.x = r.x - max(inset.z, inset.y);

    // bottom right corner
    r.y = r.y - max(inset.z, inset.w);

    // bottom left corner
    r.z = r.z - max(inset.x, inset.w); 

    // top left corner
    r.w = r.w - max(inset.x, inset.y);

    let half_size = inner_size * 0.5;
    let min = min(half_size.x, half_size.y);
    
    r = min(max(r, vec4<f32>(0.0)), vec4<f32>(min));

    return sd_rounded_box(inner_point, inner_size, r);
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    // textureSample can only be called in unform control flow, not inside an if branch.
    var color = textureSample(sprite_texture, sprite_sampler, in.uv);    
    let point = (in.uv - vec2<f32>(0.5, 0.5)) * in.size;
    switch in.mode {
        // Textured rect
        case default, 0u: {
            let distance = sd_rounded_rect(point, in.size, in.radius);
            return mix(in.color * color, vec4<f32>(0.0), smoothstep(-1.0, 1.0, distance));
        }
        // Untextured rect
        case 1u: {
            let distance = sd_rounded_rect(point, in.size, in.radius);
            return mix(in.color, vec4<f32>(0.0), smoothstep(-1.0, 1.0, distance));
        }
        // Untextured border
        case 2u: {
            let distance = sd_rounded_border(point, in.size, in.radius, in.thickness);
            return mix(in.color, vec4<f32>(0.0), smoothstep(-1.0, 1.0, distance));
        }
    }
}