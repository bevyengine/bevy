#import bevy_render::view  View

@group(0) @binding(0)
var<uniform> view: View;

struct VertexOutput {
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) @interpolate(flat) size: vec2<f32>,
    @location(3) @interpolate(flat) mode: u32,
    @location(4) @interpolate(flat) radius: vec4<f32>,    
    @location(5) @interpolate(flat) thickness: vec4<f32>,    
    @builtin(position) position: vec4<f32>,
};

@vertex
fn vertex(
    @location(0) vertex_position: vec3<f32>,
    @location(1) vertex_uv: vec2<f32>,
    @location(2) vertex_color: vec4<f32>,
    @location(3) mode: u32,
    @location(4) radius: vec4<f32>,
    @location(5) thickness: vec4<f32>,
    @location(6) size: vec2<f32>,
) -> VertexOutput {
    var out: VertexOutput;
    out.uv = vertex_uv;
    out.position = view.view_proj * vec4<f32>(vertex_position, 1.0);
    out.color = vertex_color;
    out.mode = mode;
    out.radius = radius;
    out.size = size;
    out.thickness = thickness;
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

fn sd_inner_box(point: vec2<f32>, size: vec2<f32>, thickness: vec4<f32>) -> f32 {
    let p = point + 0.5 * (thickness.xy - thickness.zw);
    let size = size - thickness.xy - thickness.zw;
    let d = abs(p) - 0.5 * size;
    return length(max(d, vec2<f32>(0.0))) + min(max(d.x, d.y), 0.0);
}

fn sd_inset_rounded_rect(point: vec2<f32>, size: vec2<f32>, radius: vec4<f32>, inset: vec4<f32>) -> f32 {
    var r = radius;
    // top right corner
    r.y = r.y - max(inset.z, inset.y);

    // bottom right corner
    r.x = r.x - max(inset.z, inset.w);

    // bottom left corner
    r.w = r.w - max(inset.x, inset.w); 

    // top left corner
    r.z = r.z - max(inset.x, inset.y);
    
    r = max(r, vec4<f32>(0.0));
    let p = point + 0.5 * (inset.zw - inset.xy);
    let inner_size = size - inset.xy - inset.zw;
    return sd_rounded_rect(p, inner_size, r);
}

fn sd_box(point: vec2<f32>, size: vec2<f32>) -> f32 {
    let d = abs(point) - 0.5 * size;
    return length(max(d,vec2<f32>(0.0))) + min(max(d.x,d.y),0.0);
}

fn sd_outer_box(point: vec2<f32>, size: vec2<f32>, inset: vec4<f32>) -> f32 {
    return -sd_inner_box(point, size, inset);
}

fn sd_box_border(point: vec2<f32>, size: vec2<f32>, thickness: vec4<f32>) -> f32 {
    return max(sd_box(point, size), sd_inner_box(point, size, thickness));
}

fn sd_rounded_border(point: vec2<f32>, size: vec2<f32>, radius: vec4<f32>, thickness: vec4<f32>) -> f32 {
    let a = sd_rounded_rect(point, size, radius);
    let b = sd_inset_rounded_rect(point, size, radius, thickness);
    return max(a, -b);
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    // textureSample can only be called in unform control flow, not inside an if branch.
    var color = textureSample(sprite_texture, sprite_sampler, in.uv);    
        switch in.mode {
        // Textured rect
        case default, 0u: {
            return in.color * color;
        }
        // Untextured rect
        case 1u: {
            let point = (in.uv - vec2<f32>(0.5, 0.5)) * in.size;
            let distance = sd_rounded_rect(point, in.size, in.radius);
            return mix(in.color, vec4<f32>(0.0), smoothstep(-0.5, 0.5, distance));
        }
        // Untextured border
        case 2u: {
            let point = (in.uv - vec2<f32>(0.5, 0.5)) * in.size;
            let distance = sd_rounded_border(point, in.size, in.radius, in.thickness);
            return mix(in.color, vec4<f32>(0.0), smoothstep(-0.5, 0.5, distance));
        }
    }
}

