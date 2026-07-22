#define_import_path bevy_ui::ui_node

#import bevy_render::view::View

const TEXTURED = 1u;
const RIGHT_VERTEX = 2u;
const BOTTOM_VERTEX = 4u;
// must align with BORDER_* shader_flags from bevy_ui/render/mod.rs
const BORDER_LEFT: u32 = 256u;
const BORDER_TOP: u32 = 512u;
const BORDER_RIGHT: u32 = 1024u;
const BORDER_BOTTOM: u32 = 2048u;
const BORDER_ANY: u32 = BORDER_LEFT + BORDER_TOP + BORDER_RIGHT + BORDER_BOTTOM;
const INVERT: u32 = 4096u;

fn enabled(flags: u32, mask: u32) -> bool {
    return (flags & mask) != 0u;
}

@group(0) @binding(0) var<uniform> view: View;

struct VertexOutput {
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,

    @location(2) @interpolate(flat) size: vec2<f32>,
    @location(3) @interpolate(flat) flags: u32,
    @location(4) @interpolate(flat) radius_x: vec4<f32>,
    @location(5) @interpolate(flat) radius_y: vec4<f32>,
    @location(6) @interpolate(flat) border: vec4<f32>,

    // Position relative to the center of the rectangle.
    @location(7) point: vec2<f32>,
    @builtin(position) position: vec4<f32>,
};

@vertex
fn vertex(
    @location(0) vertex_position: vec3<f32>,
    @location(1) vertex_uv: vec2<f32>,
    @location(2) vertex_color: vec4<f32>,
    @location(3) flags: u32,

    // x: top left, y: top right, z: bottom right, w: bottom left.
    @location(4) radius_x: vec4<f32>,
    @location(5) radius_y: vec4<f32>,

    // x: left, y: top, z: right, w: bottom.
    @location(6) border: vec4<f32>,
    @location(7) size: vec2<f32>,
    @location(8) point: vec2<f32>,
) -> VertexOutput {
    var out: VertexOutput;
    out.uv = vertex_uv;
    out.position = view.clip_from_world * vec4(vertex_position, 1.0);
    out.color = vertex_color;
    out.flags = flags;
    out.radius_x = radius_x;
    out.radius_y = radius_y;
    out.size = size;
    out.border = border;
    out.point = point;

    return out;
}

@group(1) @binding(0) var sprite_texture: texture_2d<f32>;
@group(1) @binding(1) var sprite_sampler: sampler;


// Returns the radius of the corner closest to the given point.
//
// Arguments:
//  - `point`          -> The point used to choose the closest corner.
//  - `corner_radii_x` -> The horizontal radius of each rounded corner.
//  - `corner_radii_y` -> The vertical radius of each rounded corner.
//                        Both ordered x: top left, y: top right, z: bottom right, w: bottom left.
fn select_corner_radius(
    point: vec2<f32>,
    corner_radii_x: vec4<f32>,
    corner_radii_y: vec4<f32>,
) -> vec2<f32> {
    // If 0.0 < y then select bottom left (w) and bottom right corner radius (z).
    // Else select top left (x) and top right corner radius (y).
    let rxs = select(corner_radii_x.xy, corner_radii_x.wz, 0.0 < point.y);
    let rys = select(corner_radii_y.xy, corner_radii_y.wz, 0.0 < point.y);
    // w and z are swapped above so that both pairs are in left to right order, otherwise this second 
    // select statement would return the incorrect value for the bottom pair.
    return vec2(select(rxs.x, rxs.y, 0.0 < point.x), select(rys.x, rys.y, 0.0 < point.x));
}

// The returned value is the shortest distance from the given point to the boundary of the rounded 
// box.
// 
// Negative values indicate that the point is inside the rounded box, positive values that the point 
// is outside, and zero is exactly on the boundary.
//
// Arguments: 
//  - `point`           -> The function will return the distance from this point to the closest point on 
//                          the boundary.
//  - `size`            -> The maximum width and height of the box.
//  - `corner_radii_x`   -> The horizontal semi-axis of each rounded corner. Ordered counter clockwise starting top left.
//  - `corner_radii_y`   -> The vertical semi-axis of each rounded corner. Ordered counter clockwise starting top left.
fn sd_rounded_box(
    point: vec2<f32>,
    size: vec2<f32>,
    corner_radii_x: vec4<f32>,
    corner_radii_y: vec4<f32>,
) -> f32 {
    let radius = select_corner_radius(point, corner_radii_x, corner_radii_y);
    // Vector from the corner closest to the point, to the point.
    let corner_to_point = abs(point) - 0.5 * size;
    let straight_distance = max(corner_to_point.x, corner_to_point.y);
    if min(radius.x, radius.y) <= 0.0 {
        return straight_distance;
    }
    // Vector from the center of the corner ellipse to the point.
    let q = corner_to_point + radius;
    let edge_distance = max(q.x - radius.x, q.y - radius.y);
    let inv_radii_sq = 1.0 / (radius * radius);
    let corner_distance = distance_to_ellipse_approx(q, inv_radii_sq, 1.0); 
    return select(edge_distance, corner_distance, q.x > 0.0 && q.y > 0.0);
}

fn sd_inset_rounded_box(
    point: vec2<f32>,
    size: vec2<f32>,
    radius_x: vec4<f32>,
    radius_y: vec4<f32>,
    inset: vec4<f32>,
) -> f32 {
    let inner_size = size - inset.xy - inset.zw;
    let inner_center = inset.xy + 0.5 * inner_size - 0.5 * size;
    let inner_point = point - inner_center;

    var rx = radius_x;
    var ry = radius_y;

    // Top left corner.
    rx.x = rx.x - inset.x;
    ry.x = ry.x - inset.y;

    // Top right corner.
    rx.y = rx.y - inset.z;
    ry.y = ry.y - inset.y;

    // Bottom right corner.
    rx.z = rx.z - inset.z;
    ry.z = ry.z - inset.w;

    // Bottom left corner.
    rx.w = rx.w - inset.x;
    ry.w = ry.w - inset.w;

    let half_size = inner_size * 0.5;

    rx = min(max(rx, vec4(0.0)), vec4<f32>(half_size.x));
    ry = min(max(ry, vec4(0.0)), vec4<f32>(half_size.y));
    let is_zero_radius = min(rx, ry) <= vec4(0.0);
    rx = select(rx, vec4(0.0), is_zero_radius);
    ry = select(ry, vec4(0.0), is_zero_radius);

    return sd_rounded_box(inner_point, inner_size, rx, ry);
}

fn nearest_border_active(point_vs_mid: vec2<f32>, size: vec2<f32>, width: vec4<f32>, flags: u32) -> bool {
    if (flags & BORDER_ANY) == BORDER_ANY {
        return true;
    }
 
    // get point vs top left
    let point = clamp(point_vs_mid + size * 0.49999, vec2(0.0), size);
 
    let left = point.x / width.x;
    let top = point.y / width.y;
    let right = (size.x - point.x) / width.z;
    let bottom = (size.y - point.y) / width.w;
 
    let min_dist = min(min(left, top), min(right, bottom));
 
    return (enabled(flags, BORDER_LEFT) && min_dist == left) ||
        (enabled(flags, BORDER_TOP) && min_dist == top) || 
        (enabled(flags, BORDER_RIGHT) && min_dist == right) || 
        (enabled(flags, BORDER_BOTTOM) && min_dist == bottom);
}

// get alpha for antialiasing for sdf
fn antialias(distance: f32) -> f32 {
    // Using the fwidth(distance) was causing artifacts, so just use the distance.
    return saturate(0.5 - distance);
}

fn draw_uinode_border(
    color: vec4<f32>,
    point: vec2<f32>,
    size: vec2<f32>,
    radius_x: vec4<f32>,
    radius_y: vec4<f32>,
    border: vec4<f32>,
    flags: u32,
) -> vec4<f32> {
    // Signed distances. The magnitude is the distance of the point from the edge of the shape.
    // * Negative values indicate that the point is inside the shape.
    // * Zero values indicate the point is on the edge of the shape.
    // * Positive values indicate the point is outside the shape.

    // Signed distance from the exterior boundary.
    let external_distance = sd_rounded_box(point, size, radius_x, radius_y);

    // Signed distance from the border's internal edge (the signed distance is negative if the point 
    // is inside the rect but not on the border).
    // If the border size is set to zero, this is the same as the external distance.
    let internal_distance = sd_inset_rounded_box(point, size, radius_x, radius_y, border);

    // Signed distance from the border (the intersection of the rect with its border).
    // Points inside the border have negative signed distance. Any point outside the border, whether 
    // outside the outside edge, or inside the inner edge have positive signed distance.
    let border_distance = max(external_distance, -internal_distance);

    // check if this node should apply color for the nearest border
    let nearest_border = select(0.0, 1.0, nearest_border_active(point, size, border, flags));

#ifdef ANTI_ALIAS
    // At external edges with no border, `border_distance` is equal to zero. 
    // This select statement ensures we only perform anti-aliasing where a non-zero width border 
    // is present, otherwise an outline about the external boundary would be drawn even without 
    // a border.
    let t = select(1.0 - step(0.0, border_distance), antialias(border_distance), external_distance < internal_distance);
#else
    let t = 1.0 - step(0.0, border_distance);
#endif

    // Blend mode ALPHA_BLENDING is used for UI elements, so we don't premultiply alpha here.
    return vec4(color.rgb, saturate(color.a * t * nearest_border));
}

fn draw_uinode_background(
    color: vec4<f32>,
    point: vec2<f32>,
    size: vec2<f32>,
    radius_x: vec4<f32>,
    radius_y: vec4<f32>,
    border: vec4<f32>,
    flags: u32,
) -> vec4<f32> {
    // When drawing the background only draw the internal area and not the border.
    let internal_distance = sd_inset_rounded_box(point, size, radius_x, radius_y, border) * select(1., -1, enabled(flags, INVERT));

#ifdef ANTI_ALIAS
    let t = antialias(internal_distance);
#else
    let t = 1.0 - step(0.0, internal_distance);
#endif

    return vec4(color.rgb, saturate(color.a * t));
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let texture_color = textureSample(sprite_texture, sprite_sampler, in.uv);

    // Only use the color sampled from the texture if the `TEXTURED` flag is enabled. 
    // This allows us to draw both textured and untextured shapes together in the same batch.
    let color = select(in.color, in.color * texture_color, enabled(in.flags, TEXTURED));

    if enabled(in.flags, BORDER_ANY) {
        return draw_uinode_border(color, in.point, in.size, in.radius_x, in.radius_y, in.border, in.flags);
    } else {
        return draw_uinode_background(color, in.point, in.size, in.radius_x, in.radius_y, in.border, in.flags);
    }
}

// One iteration of Newton's method on the 2D equation of an ellipse:  
//  
//     E(x, y) = x^2/a^2 + y^2/b^2 - 1  
//  
// The Jacobian of this equation is:  
//  
//     J(E(x, y)) = [ 2*x/a^2 2*y/b^2 ]  
//  
// We approximate the distance with:  
//  
//     E(x, y) / ||J(E(x, y))||  
//  
// See G. Taubin, "Distance Approximations for Rasterizing Implicit  
// Curves", section 3.  
//  
// A scale relative to the unit scale of the ellipse may be passed in to cause  
// the math to degenerate to length(p) when scale is 0, or otherwise give the  
// normal distance approximation if scale is 1.
fn distance_to_ellipse_approx(p: vec2<f32>, inv_radii_sq: vec2<f32>, scale: f32) -> f32 {
    let p_r = p * inv_radii_sq;
    let g = dot(p, p_r) - scale;
    let dG = (1.0 + scale) * p_r;
    return g * inverseSqrt(dot(dG, dG));
}