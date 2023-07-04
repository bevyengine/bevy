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
    @location(6) border_color: vec4<f32>,
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
    @location(7) border_color: vec4<f32>,
) -> VertexOutput {
    var out: VertexOutput;
    out.uv = vertex_uv;
    out.position = view.view_proj * vec4<f32>(vertex_position, 1.0);
    out.color = vertex_color;
    out.border_color = border_color;
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

fn sd_box(point: vec2<f32>, size: vec2<f32>) -> f32 {
    let d = abs(point) - 0.5 * size;
    return length(max(d,vec2<f32>(0.0))) + min(max(d.x,d.y),0.0);
}

fn sd_rounded_rect(point: vec2<f32>, size: vec2<f32>, radius: vec4<f32>) -> f32 {
    let top_right_radius = radius.x;
    let bottom_right_radius = radius.y;
    let bottom_left_radius = radius.z;
    let top_left_radius = radius.w;
    var r: f32;
    if 0.0 < point.x {
        if 0.0 < point.y {
            r = top_left_radius;
        } else {
            r = bottom_left_radius;
        }
    } else {
        if 0.0 < point.y {
            r = top_right_radius;
        } else {
            r = bottom_right_radius;
        }
    }
    let q = abs(point) - 0.5 * size + r;
    return min(max(q.x, q.y), 0.0) + length(max(q, vec2(0.0, 0.0))) - r;
}

fn calculate_inner_radius(radius: f32, inset: vec2<f32>) -> f32 {
    let s = max(inset.x, inset.y);
    return min(radius - s, 0.0);
}

fn sd_inset_rounded_rect(point: vec2<f32>, size: vec2<f32>, radius: vec4<f32>, inset: vec4<f32>) -> f32 {
    let left = inset.x;
    let top = inset.y;
    let right = inset.z;
    let bottom = inset.w;

    let top_right = inset.zy;
    let bottom_right = inset.zw;
    let bottom_left = inset.xw;
    let top_left = inset.xy;
    
    let top_right_radius = radius.x;
    let bottom_right_radius = radius.y;
    let bottom_left_radius = radius.z;
    let top_left_radius = radius.w;

    let inner_size = size - inset.xy - inset.zw;
    let inner_center = inset.xy + 0.5 * inner_size;
    let inner_point = point - inner_center;

    var inner_radius: vec4<f32>;
    inner_radius.x = calculate_inner_radius(top_right_radius, top_right);
    inner_radius.y = calculate_inner_radius(bottom_right_radius, bottom_right);
    inner_radius.z = calculate_inner_radius(bottom_left_radius, bottom_left);
    inner_radius.z = calculate_inner_radius(top_left_radius, top_left);

    return sd_rounded_rect(inner_point, inner_size, inner_radius);
}

fn sd_rounded_border(point: vec2<f32>, size: vec2<f32>, radius: vec4<f32>, thickness: vec4<f32>) -> f32 {
    let a = sd_rounded_rect(point, size, radius);
    let b = sd_inset_rounded_rect(point, size, radius, thickness);
    return max(a, -b);
}

fn sd_inset_rect(point: vec2<f32>, size: vec2<f32>, radius: vec4<f32>, inset: vec4<f32>) -> f32 {
    let inner_size = size - inset.xy - inset.zw;
    let inner_center = inset.xy + 0.5 * inner_size;
    let inner_point = point - inner_center;
    let inner_radius = vec4<f32>(0.0);

    return sd_rounded_rect(inner_point, inner_size, inner_radius);
}

fn rects(in: VertexOutput) -> vec4<f32> {
    var outer_color = vec4<f32>(1.0, 0.0, 0.0, 1.0); // red
    var border_color = vec4<f32>(0.0, 1.0, 0.0, 1.0); // green
    var inner_color = vec4<f32>(0.0, 0.0, 1.0, 1.0); // blue

    let point = (in.uv - vec2<f32>(0.5)) * in.size;
    let outer_distance = sd_box(point, in.size);
    if outer_distance <= 0.0 {
        return outer_color;
    }

    return border_color;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    return rects(in);
}

//     // textureSample can only be called in unform control flow, not inside an if branch.
    
//     //var inner_color = in.color * select(vec4<f32>(1.0), textureSample(sprite_texture, sprite_sampler, in.uv), in.mode == 0u);    

//     var outer_color = vec4<f32>(1.0, 0.0, 0.0, 1.0); // red
//     var border_color = vec4<f32>(0.0, 1.0, 0.0, 1.0); // green
//     var inner_color = vec4<f32>(0.0, 0.0, 1.0, 1.0); // blue

//     // displacement from center of rect
//     let point = (in.uv - vec2<f32>(0.5)) * in.size;
//     // distance from border edge
//     let outer_distance = sd_rounded_rect(point, in.size, in.radius);
//     //let inner_distance = sd_inset_rounded_rect(point, in.size, in.radius, in.thickness);
//     let inner_distance = sd_inset_rect(point, in.size, in.radius, in.thickness);
//     let fwidth_outer = fwidth(outer_distance);
//     let fwidth_inner = fwidth(inner_distance);

//     if inner_distance <= 0.0 {
//              // inside border inner edge
//          //return mix( in.border_color, inner_color, smoothstep(-fwidth_inner, 0.0, inner_distance));
//          return inner_color;
//      }

//     if outer_distance <= 0.0 {
//         return border_color;
//     }

//     // if inner_distance <= 0.0 {
//     //     // inside border inner edge
//     //     //return mix( in.border_color, inner_color, smoothstep(-fwidth_inner, 0.0, inner_distance));
//     //     return inner_color;
//     // }

//     // if outer_distance <= 0.0 {
//     //     // inside border outer edge
//     //     return border_color;
//     // }

//     // if 0. < outer_distance {
//     //     // outside outer border, no color

//     //     if 0. < inner_distance {
//     //         let a = mix(inner_color.w, 0.0, smoothstep(0.0, fwidth_outer, outer_distance));
//     //         return vec4<f32>(inner_color.xyz, a);
//     //     }

//     //     let a = mix(in.border_color.w, 0.0, smoothstep(0.0, fwidth_outer, outer_distance));
//     //     return vec4<f32>(in.border_color.xyz, a);
//     //     // } else {
//     //     //     let a = mix(inner_color.w, 0.0, smoothstep(0.0, fwidth_outer, outer_distance));
//     //     //     return vec4<f32>(inner_color.xyz, a);
//     //     // }    
//     // }

//     //return mix(inner_color, in.border_color, smoothstep(-fwidth_inner, 0.0, inner_distance));

//     return outer_color;


    

//     // switch in.mode {
//     //     // Textured rect
//     //     case default, 0u: {
//     //         let distance = sd_rounded_rect(point, in.size, in.radius);
//     //         return mix(in.color * color, vec4<f32>(0.0), smoothstep(-1.0, 1.0, distance));
//     //     }
//     //     // Untextured rect
//     //     case 1u: {
//     //         let distance = sd_rounded_rect(point, in.size, in.radius);
//     //         return mix(in.color, vec4<f32>(0.0), smoothstep(-1.0, 1.0, distance));
//     //     }
//     //     // Untextured border
//     //     case 2u: {
//     //         let distance = sd_rounded_border(point, in.size, in.radius, in.thickness);
//     //         return mix(in.border_color, color, smoothstep(-1.0, 1.0, distance));
//     //     }
//     // }
// }

