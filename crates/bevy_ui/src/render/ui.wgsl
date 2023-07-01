#import bevy_render::view  View

@group(0) @binding(0)
var<uniform> view: View;

struct VertexOutput {
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) @interpolate(flat) size: vec2<f32>,
    @location(3) @interpolate(flat) mode: u32,
    @location(4) @interpolate(flat) radius: vec4<f32>,    
    @location(5) @interpolate(flat) border: vec4<f32>,    
    @builtin(position) position: vec4<f32>,
};

@vertex
fn vertex(
    @location(0) vertex_position: vec3<f32>,
    @location(1) vertex_uv: vec2<f32>,
    @location(2) vertex_color: vec4<f32>,
    @location(3) mode: u32,
    @location(4) radius: vec4<f32>,
    @location(5) border: vec4<f32>,
    @location(6) size: vec2<f32>,
) -> VertexOutput {
    var out: VertexOutput;
    out.uv = vertex_uv;
    out.position = view.view_proj * vec4<f32>(vertex_position, 1.0);
    out.color = vertex_color;
    out.mode = mode;
    out.radius = radius;
    out.size = size;
    out.border = border;
    return out;
}

@group(1) @binding(0)
var sprite_texture: texture_2d<f32>;
@group(1) @binding(1)
var sprite_sampler: sampler;

fn sd_rounded_rect(point: vec2<f32>, size: vec2<f32>, radius: vec4<f32>) -> f32 {
    var r: f32;
    if 0.0 < point.x {
        if 0.0 < point.y {
            r = radius.y;
        } else {
            r = radius.x;
        }
    } else {
        if 0.0 < point.y {
            r = radius.z;
        } else {
            r = radius.w;
        }
    }
    let q = abs(point) - 0.5 * size + r;
    return min(max(q.x, q.y), 0.0) + length(max(q, vec2(0.0, 0.0))) - r;
}

fn sd_inset_rounded_rect(point: vec2<f32>, size: vec2<f32>, radius: vec4<f32>, inset: vec4<f32>) -> f32 {
    var r = radius;

    // top right corner
    r.x = r.x - max(inset.z, inset.y);

    // bottom right corner
    r.y = r.y - max(inset.z, inset.w);

    // bottom left corner
    r.z = r.z - max(inset.x, inset.w); 

    // top left corner
    r.w = r.w - max(inset.x, inset.y);
    
    r = max(r, vec4<f32>(0.0));
    let p = point + 0.5 * (inset.zw - inset.xy);
    let inner_size = size - inset.xy - inset.zw;
    return sd_rounded_rect(p, inner_size, r);
}

fn sd_rounded_border(point: vec2<f32>, size: vec2<f32>, radius: vec4<f32>, border: vec4<f32>) -> f32 {
    let a = sd_rounded_rect(point, size, radius);
    let b = sd_inset_rounded_rect(point, size, radius, border);
    return max(a, -b);
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    // textureSample can only be called in unform control flow, not inside an if branch.
    let color = in.color * select(vec4<f32>(1.0), textureSample(sprite_texture, sprite_sampler, in.uv), in.mode == 0u);    
    let point = (in.uv - vec2<f32>(0.5, 0.5)) * in.size;
    var outer_radius : f32;
    var inset: vec2<f32>;
    if 0.0 < point.x {
        if 0.0 < point.y {
            outer_radius = in.radius.y;
            inset = in.border.zw;
        } else {
            outer_radius = in.radius.x;
            inset = in.border.zy;
        }
    } else {
        if 0.0 < point.y {
            outer_radius = in.radius.z;
            inset = in.border.xw;
        } else {
            outer_radius = in.radius.w;
            inset = in.border.xy;
        }
    }    
    let inner_radius = outer_radius - max(inset.x, inset.y);
    let q = abs(point) - 0.5 * in.size + outer_radius;
    let outer_distance = min(max(q.x, q.y), 0.0) + length(max(q, vec2(0.0, 0.0))) - outer_radius;
    let p = point + 0.5 * (in.border.zw - in.border.xy);
    let inner_size = in.size - in.border.xy - in.border.zw;
    let q2 = abs(p) - 0.5 * inner_size + inner_radius;
    let interior_distance = min(max(q2.x, q2.y), 0.0) + length(max(q2, vec2(0.0, 0.0))) - inner_radius;
    let distance = select(outer_distance, max(outer_distance, -interior_distance), in.mode == 2u);
    let f = 0.5 * fwidth(distance);
    if  max(q.x, q.y) > 0.0  {
        let a = mix(color.w, 0.0, smoothstep(-f, f, distance));
        return vec4<f32>(color.xyz, a);
    }

    let a = mix(color.w, 0.0, step(0.0, distance));
    return vec4<f32>(color.xyz, a);
}

