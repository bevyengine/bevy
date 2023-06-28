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
            r = radius.x;
        } else {
            r = radius.y;
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
    r.x = r.x - max(inset.y, inset.z);
    r.y = r.y - max(inset.y, inset.w);
    r.z = r.z - max(inset.x, inset.z);
    r.w = r.w - max(inset.x, inset.w); 
    r = max(r, vec4<f32>(0.0));
    let p = point + 0.5 * (inset.xy - inset.zw);
    let size = size - inset.xy - inset.zw;
    let d = abs(p) - 0.5 * size;
    return sd_rounded_rect(p, size, r);
}

fn sd_box(point: vec2<f32>, size: vec2<f32>) -> f32 {
    let d = abs(point) - 0.5 * size;
    return length(max(d,vec2<f32>(0.0))) + min(max(d.x,d.y),0.0);
}

fn sd_outer_box(point: vec2<f32>, size: vec2<f32>, inset: vec4<f32>) -> f32 {
    // let h = thickness.x + thickness.y;
    // let v = thickness.z + thickness.w;
    return -sd_inner_box(point, size, inset);
}

// @fragment
// fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
//     // textureSample can only be called in unform control flow, not inside an if branch.
//     var color = textureSample(sprite_texture, sprite_sampler, in.uv);

//     let point = (in.uv - vec2<f32>(0.5, 0.5)) * in.size;
    
//     let d = abs(sd_rounded_rect(point, in.size, in.radius)) - 1.;
//     switch in.mode {
//         // Textured rect
//         case 0u: {
//             return mix(in.color * color, vec4<f32>(0.0), smoothstep(-1.0, 0.5, d));
//         }
//         // Untextured rect
//         case 1u: {
//             return mix(in.color, vec4<f32>(0.0), smoothstep(-1.0, 0.5, d));
//         }
//         // Inverted rect (fill outside the rounded corners)
//         case 2u, default: {
//             return mix(vec4<f32>(0.0), in.color, smoothstep(-1.0, 1.0, d));
//         }
//     }
// }

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
    // let d = sd_rounded_rect(point, in.size, in.radius);
    // let delta = fwidth(d);
    // if -20. <= d && d <= 0.  {
    //     return in.color;
    // }

    //let d = sd_border_box(point, in.size, vec4<f32>(4.0));//in.thickness);
    //let distance = sd_outer_box(point, in.size, in.thickness);
    // if d < 0. {
    //     return in.color;
    // }

    //return vec4<f32>(0.0);
    switch in.mode {
        // Textured rect
        case 0u: {
            return in.color * color;
        }
        // Untextured rect
        case 1u: {
            let point = (in.uv - vec2<f32>(0.5, 0.5)) * in.size;
            let distance = sd_rounded_rect(point, in.size, in.radius);
            return in.color * step(distance, 0.0);
        }
        // Untextured border
        case 2u, default: {
            let point = (in.uv - vec2<f32>(0.5, 0.5)) * in.size;
            let distance = sd_rounded_border(point, in.size, in.radius, in.thickness);
            //let distance = sd_inset_rounded_rect(point, in.size, in.radius, in.thickness);          
            //let distance = sd_rounded_rect(point, in.size, in.radius);
            //return in.color * step(distance, 0.);
            return mix(in.color, vec4<f32>(0.0), smoothstep(-0.5, 0.5, distance));
        }
    }
}

