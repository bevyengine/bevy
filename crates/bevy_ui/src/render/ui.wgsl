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
    @location(5) @interpolate(flat) border: vec4<f32>,    
    @location(6) border_color: vec4<f32>,
    @location(7) point: vec2<f32>,
    @builtin(position) position: vec4<f32>,
};

@vertex
fn vertex(
    @location(0) vertex_position: vec3<f32>,
    @location(1) vertex_uv: vec2<f32>,
    @location(2) vertex_color: vec4<f32>,
    @location(3) flags: u32,
    @location(4) radius: vec4<f32>,
    @location(5) border: vec4<f32>,
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
    out.border = border;

    // Multiplying by 0.49999 instead of 0.5 is a hack that stops borders stepping across a pixel when they go directly through the center. Should be a better way to fix this?
    out.point = (vertex_uv - vec2<f32>(0.49999)) * size;
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
    var internal_color = select(in.color, in.color * textureSample(sprite_texture, sprite_sampler, in.uv), (in.flags & TEXTURED_QUAD) != 0u);

    // Distance from external border. Positive values inside.
    let external_distance = sd_rounded_box(in.point, in.size, in.radius);

    // Distance from internal border. Positive values inside.
    let internal_distance = sd_inset_rounded_box(in.point, in.size, in.radius, in.border);

    // Distance from border, positive values inside border.
    let border = max(-internal_distance, external_distance);

    // Distance from interior, positive values inside interior.
    let interior = max(internal_distance, external_distance);
    
    // Distance from exterior, positive values outside node.
    let exterior = -external_distance;

    // Anti-aliasing
    let fborder = 0.5 * fwidth(border);
    let fexternal = 0.5 * fwidth(external_distance);
    let p = smoothstep(-fborder, fborder, border);
    let q = smoothstep(-fexternal, fexternal, external_distance);

    if (in.flags & BOX_SHADOW) != 0u {
        var rect_dist = 1.0 - sigmoid(sd_rounded_box(in.point,in.size,in.radius));
        let color = in.color.rgb;
        return vec4(color, in.color.a * rect_dist * 1.42);
    }

    if interior < exterior {
        if border < exterior {
            return mix(in.border_color, internal_color, p);
        } else {
            let a = mix(0., internal_color.a, p);
            return vec4<f32>(internal_color.rgb, a);
        }
    }

    var boundary_color = select(internal_color, in.border_color, border < interior);
    let a = mix(boundary_color.a, 0., q);
    return vec4<f32>(boundary_color.rgb, a);
}