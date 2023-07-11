#import bevy_render::view  View

const TEXTURED_QUAD = 1u;
const BOX_SHADOW = 2u;
const DISABLE_AA = 4u;
const RIGHT_VERTEX = 8u;
const BOTTOM_VERTEX = 16u;
const BORDER: u32 = 32u;

@group(0) @binding(0)
var<uniform> view: View;

struct VertexOutput {
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) @interpolate(flat) size: vec2<f32>,
    @location(3) @interpolate(flat) flags: u32,
    @location(4) @interpolate(flat) radius: vec4<f32>,    
    @location(5) @interpolate(flat) border: vec4<f32>,    
    
    // position relative to the center of the rectangle
    @location(6) point: vec2<f32>,
    @builtin(position) position: vec4<f32>,
};

@vertex
fn vertex(
    @location(0) vertex_position: vec3<f32>,
    @location(1) vertex_uv: vec2<f32>,
    @location(2) vertex_color: vec4<f32>,
    @location(3) flags: u32,

    // radius.x = top left radius, .y = top right, .z = bottom right, .w = bottom left
    @location(4) radius: vec4<f32>,

    // border.x = left width, .y = top, .z = right, .w = bottom
    @location(5) border: vec4<f32>,

    @location(6) size: vec2<f32>,
) -> VertexOutput {
    var out: VertexOutput;
    out.uv = vertex_uv;
    out.position = view.view_proj * vec4<f32>(vertex_position, 1.0);
    out.color = vertex_color;
    out.flags = flags;
    out.radius = radius;
    out.size = size;
    out.border = border;
    var point = 0.49999 * size;
    if (flags & RIGHT_VERTEX) == 0u {
        point.x *= -1.;
    }
    if (flags & BOTTOM_VERTEX) == 0u {
        point.y *= -1.;
    }
    out.point = point;

    return out;
}

@group(1) @binding(0)
var sprite_texture: texture_2d<f32>;

@group(1) @binding(1)
var sprite_sampler: sampler;

fn sigmoid(t: f32) -> f32 {
    return 1.0 / (1.0 + exp(-t));
}

// The returned value is the shortest distance from the given point to the boundary of the rounded box.
// Negative values indicate that the point is inside the rounded box, positive values that the point is outside, and zero is exactly on the boundary.
// arguments
// point -> The function will return the distance from this point to the closest point on the boundary.
// size -> The maximum width and height of the box.
// corner_radii -> The radius of each rounded corner. Ordered counter clockwise starting top left:
//                      x = top left, y = top right, z = bottom right, w = bottom left.
fn sd_rounded_box(point: vec2<f32>, size: vec2<f32>, corner_radii: vec4<f32>) -> f32 {
    // if 0.0 < y then select bottom left (w) and bottom right corner radius (z)
    // else select top left (x) and top right corner radius (y)
    let rs = select(corner_radii.xy, corner_radii.wz, 0.0 < point.y);
    // w and z are swapped so that both pairs are in left to right order, otherwise this second select statement would return the incorrect value for the bottom pair.
    let radius = select(rs.x, rs.y, 0.0 < point.x);
    // Vector from the corner closest to the point, to the point
    let corner_to_point = abs(point) - 0.5 * size;
    // Vector from the center of the radius circle to the point 
    let q = corner_to_point + radius;
    // length from center of the radius circle to the point, 0s a component if the point is not within the quadrant of the radius circle that is part of the curved corner.
    let l = length(max(q, vec2(0.0)));
    let m = min(max(q.x, q.y), 0.0);
    return l + m - radius;
}

fn sd_inset_rounded_box(point: vec2<f32>, size: vec2<f32>, radius: vec4<f32>, inset: vec4<f32>) -> f32 {
    let inner_size = size - inset.xy - inset.zw;
    let inner_center = inset.xy + 0.5 * inner_size - 0.5 *size;
    let inner_point = point - inner_center;

    var r = radius;

    // top left corner
    r.x = r.x - max(inset.x, inset.y);

    // top right corner
    r.y = r.y - max(inset.z, inset.y);

    // bottom right corner
    r.z = r.z - max(inset.z, inset.w); 

    // bottom left corner
    r.w = r.w - max(inset.x, inset.w);

    let half_size = inner_size * 0.5;
    let min = min(half_size.x, half_size.y);
    
    r = min(max(r, vec4<f32>(0.0)), vec4<f32>(min));

    return sd_rounded_box(inner_point, inner_size, r);
}

#ifdef CLAMP_INNER_CURVES
fn sd_inset_rounded_box(point: vec2<f32>, size: vec2<f32>, radius: vec4<f32>, inset: vec4<f32>) -> f32 {
    let inner_size = size - inset.xy - inset.zw;
    let inner_center = inset.xy + 0.5 * inner_size - 0.5 *size;
    let inner_point = point - inner_center;

    var r = radius;

    
    if 0. < min(inset.x, inset.y) || inset.x + inset.y <= 0. {
        // top left corner
        r.x = r.x - max(inset.x, inset.y);
    } else {
        r.x = 0.;
    }

    if 0. < min(inset.z, inset.y) || inset.z + inset.y <= 0.{
        // top right corner
        r.y = r.y - max(inset.z, inset.y);
    } else {
        r.y = 0.;
    }

    if 0. < min(inset.z, inset.w) || inset.z + inset.w <= 0. {
        // bottom right corner
        r.z = r.z - max(inset.z, inset.w); 
    } else {
        r.z = 0.;
    }

    if 0. < min(inset.x, inset.w) || inset.x + inset.w <= 0. {
        // bottom left corner
        r.w = r.w - max(inset.x, inset.w);
    } else {
        r.w = 0.;
    }

    let half_size = inner_size * 0.5;
    let min = min(half_size.x, half_size.y);
    
    r = min(max(r, vec4<f32>(0.0)), vec4<f32>(min));

    return sd_rounded_box(inner_point, inner_size, r);
}
#endif

const RED: vec4<f32> = vec4<f32>(1., 0., 0., 1.);
const GREEN: vec4<f32> = vec4<f32>(0., 1., 0., 1.);
const BLUE: vec4<f32> = vec4<f32>(0., 0., 1., 1.);
const WHITE = vec4<f32>(1., 1., 1., 1.);
const BLACK = vec4<f32>(0., 0., 0., 1.);

// draw the border in white, rest of rect black
fn draw_border(in: VertexOutput) -> vec4<f32> {
    // Distance from external border. Positive values outside.
    let external_distance = sd_rounded_box(in.point, in.size, in.radius);

    // Distance from internal border. Positive values inside.
    let internal_distance = sd_inset_rounded_box(in.point, in.size, in.radius, in.border);

    // Distance from border, positive values inside border.
    let border = max(-internal_distance, external_distance);

    if border < 0.0 {
        return WHITE;
    } else {
        return BLACK;
    }
}

// draw just the interior in white, rest of rect black
fn draw_interior(in: VertexOutput) -> vec4<f32> {
    // Distance from external border. Positive values outside.
    let external_distance = sd_rounded_box(in.point, in.size, in.radius);

    // Distance from internal border. Positive values inside.
   // let internal_distance = sd_inset_rounded_box(in.point, in.size, in.radius, in.border);

    // Distance from border, positive values inside border.

    if external_distance < 0.0 {
        return WHITE;
    } else {
        return BLACK;
    }
}

// Draw all the geometry
fn draw_test(in: VertexOutput) -> vec4<f32> {
    // Distance from external border. Negative inside
    let external_distance = sd_rounded_box(in.point, in.size, in.radius);

    // Distance from internal border.
    let internal_distance = sd_inset_rounded_box(in.point, in.size, in.radius, in.border);

    // Distance from border.
    let border = max(-internal_distance, external_distance);

    // Draw the area outside the border in green 
    if 0.0 < external_distance {
        return GREEN;
    }

    // Draw the area inside the border in white
    if border < 0.0 {
        return WHITE;
    }

    // draw the interior in blue
    if internal_distance < 0.0 {
        return BLUE;
    }

    // fill anything else with red (the presence of any red is a bug).
    return RED;
}

fn enabled(flags: u32, mask: u32) -> bool {
    return (flags & mask) != 0u;
}

fn draw_no_aa(in: VertexOutput) -> vec4<f32> {
    let texture_color = textureSample(sprite_texture, sprite_sampler, in.uv);
    let color = select(in.color, in.color * texture_color, enabled(in.flags, TEXTURED_QUAD));

    // negative value => point inside external border
    let external_boundary = sd_rounded_box(in.point, in.size, in.radius);
    // negative value => point inside internal border
    let internal_boundary = sd_inset_rounded_box(in.point, in.size, in.radius, in.border);
    // negative value => point inside border
    let border = max(external_boundary, -internal_boundary);

    if enabled(in.flags, BOX_SHADOW) {
        // copied from kayak
        var rect_dist = 1.0 - sigmoid(sd_rounded_box(in.point,in.size - in.border.x * 2.0, in.radius));
        let color = in.color.rgb;
        return vec4(color, in.color.a * rect_dist * 1.42);
    }

    if enabled(in.flags, BORDER) {
        if border < 0.0 {
            return color;
        } else {
            return vec4(0.0);
        }
    }

    if external_boundary < 0.0 {
        return color;
    }

    return vec4(0.0);
}

fn draw(in: VertexOutput) -> vec4<f32> {
    let texture_color = textureSample(sprite_texture, sprite_sampler, in.uv);
    let color = select(in.color, in.color * texture_color, enabled(in.flags, TEXTURED_QUAD));

    // negative value => point inside external border
    let external_boundary = sd_rounded_box(in.point, in.size, in.radius);
    // negative value => point inside internal border
    let internal_boundary = sd_inset_rounded_box(in.point, in.size, in.radius, in.border);
    // negative value => point inside border
    let border = max(external_boundary, -internal_boundary);

    let fborder = fwidth(border);
    let fexternal = fwidth(external_boundary);

    if enabled(in.flags, BOX_SHADOW) {
        // copied from kayak
        var rect_dist = 1.0 - sigmoid(sd_rounded_box(in.point,in.size - in.border.x * 2.0, in.radius));
        let color = in.color.rgb;
        return vec4(color, in.color.a * rect_dist * 1.42);
    }

    if enabled(in.flags, BORDER) {
        let t = 1. - select(step(0.0, border), smoothstep(0.0, fborder, border), external_boundary < internal_boundary);
        return vec4(color.rgb, t * color.a);
    }

    let t = 1. - smoothstep(-fexternal, 0.0, external_boundary);
    return vec4(color.rgb, t * color.a);
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    // var color = select(in.color, in.color * textureSample(sprite_texture, sprite_sampler, in.uv), (in.flags & TEXTURED_QUAD) != 0u);

    // // Distance from external border. Positive values outside.
    // let external_distance = sd_rounded_box(in.point, in.size, in.radius);

    // // Distance from internal border. Positive values inside.
    // let internal_distance = sd_inset_rounded_box(in.point, in.size, in.radius, in.border);

    // // Distance from border, positive values inside border.
    // let border = max(-internal_distance, external_distance);

    // // Distance from interior, positive values inside interior.
    // let interior = max(internal_distance, external_distance);
    
    // // Distance from exterior, positive values outside node.
    // let exterior = -external_distance;

    // // Anti-aliasing
    // let fborder = 0.5 * fwidth(border);
    // let fexternal = 0.5 * fwidth(external_distance);
    // let p = smoothstep(-fborder, fborder, border);
    // let q = smoothstep(-fexternal, fexternal, external_distance);

    // if (in.flags & BOX_SHADOW) != 0u {
    //     // copied from kayak
    //     var rect_dist = 1.0 - sigmoid(sd_rounded_box(in.point,in.size - in.border.x * 2.0, in.radius));
    //     let color = in.color.rgb;
    //     return vec4(color, in.color.a * rect_dist * 1.42);
    // }

    // if (in.flags & BORDER) != 0u {
    //     if 0.0 < external_distance  {
    //         // outside border
    //         let t = 1.0 - smoothstep(0.0, 10.0, external_distance);
    //         return vec4(color.rgb, t * color.a);
    //         //return vec4<f32>(1.0);
    //     } else {
    //         let t = 1.0 - smoothstep(0.0, 10.0, -internal_distance);
    //         return vec4(color.rgb, t * color.a);
    //     }
    // }

    // // if 0. < external_distance {
    // //     // The point is outside of of the outer border
    // //     if external_distance < internal_distance {
    // //         // The closest border edge has non-zero width
    // //         return vec4<f32>(0.);
    // //     } else {
    // //         return vec4(color.rgb, smoothstep(fexternal, 0.0, external_distance));  
    // //     }
    // // }

    // // return color;

    //  return vec4<f32>(0.);
    

    // // if interior < exterior {
    // //     if border < exterior {
    // //         return mix(in.border_color, internal_color, p);
    // //     } else {
    // //         let a = mix(0., color.a, p);
    // //         return vec4<f32>(color.rgb, a);
    // //     }
    // // }

    // // var boundary_color = select(internal_color, in.border_color, border < interior);
    // // let a = mix(boundary_color.a, 0., q);
    // // return vec4<f32>(boundary_color.rgb, a);

    //return white_border(in);
    // return interior(in);
    return draw(in);
}
