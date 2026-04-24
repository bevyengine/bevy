// `enable dual_source_blending;` must appear before any other global
// declaration per the WGSL spec. We keep it unconditional (rather than
// gating it on `#ifdef SUBPIXEL`) because naga_oil's preprocessor expands
// `#define_import_path` / `#import` as leading globals, which makes a
// `#ifdef`-wrapped `enable` land "after" them and trips a parse error.
// The directive is harmless when the SUBPIXEL path is inactive — naga only
// requires the corresponding DSB capability at compile time on pipelines
// that actually use `@blend_src`. See `crates/bevy_pbr/src/atmosphere/
// render_sky.wgsl` for the same pattern.
enable dual_source_blending;

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

// Tuning parameters for `fragment_subpixel`. Declared on every UI pipeline
// variant — even the non-subpixel entry points — so the view bind group
// layout is shared. The non-subpixel `fragment` entry simply doesn't
// reference this. Phase-05 will populate from the user-facing
// `SubpixelTextSettings` / `SubpixelLcdLayout` resources; phase-04 carries
// GPUI-default values baked in at [`SubpixelTextUniforms::default`] in
// `bevy_ui_render::lib.rs`.
//
// `layout_flags` discriminant (keep in sync with `SubpixelLcdLayout` in
// `bevy_text` once phase-05 adds it):
//   0 = HorizontalRgb  (atlas R, G, B in that order — identity swizzle)
//   1 = HorizontalBgr  (swap R/B — text on a BGR panel)
//   2 = VerticalRgb    (proof-of-wiring; see vertical-limitation note below)
//   3 = VerticalBgr    (proof-of-wiring)
struct SubpixelSettings {
    enhanced_contrast: f32,
    layout_flags: u32,
    // Explicit padding matches the Rust `SubpixelTextUniforms::_pad` so the
    // trailing `vec4<f32>` below lands on a 16-byte boundary per std140
    // rules. `enhanced_contrast` (4 bytes) + `layout_flags` (4 bytes) +
    // `_pad` (8 bytes) fills the leading 16-byte slot.
    _pad: vec2<f32>,
    gamma_ratios: vec4<f32>,
}
@group(0) @binding(1) var<uniform> subpixel_settings: SubpixelSettings;

const SUBPIXEL_LAYOUT_HORIZONTAL_RGB: u32 = 0u;
const SUBPIXEL_LAYOUT_HORIZONTAL_BGR: u32 = 1u;
const SUBPIXEL_LAYOUT_VERTICAL_RGB: u32 = 2u;
const SUBPIXEL_LAYOUT_VERTICAL_BGR: u32 = 3u;

struct VertexOutput {
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,

    @location(2) @interpolate(flat) size: vec2<f32>,
    @location(3) @interpolate(flat) flags: u32,
    @location(4) @interpolate(flat) radius: vec4<f32>,    
    @location(5) @interpolate(flat) border: vec4<f32>,    

    // Position relative to the center of the rectangle.
    @location(6) point: vec2<f32>,
    @builtin(position) position: vec4<f32>,
};

@vertex
fn vertex(
    @location(0) vertex_position: vec3<f32>,
    @location(1) vertex_uv: vec2<f32>,
    @location(2) vertex_color: vec4<f32>,
    @location(3) flags: u32,

    // x: top left, y: top right, z: bottom right, w: bottom left.
    @location(4) radius: vec4<f32>,

    // x: left, y: top, z: right, w: bottom.
    @location(5) border: vec4<f32>,
    @location(6) size: vec2<f32>,
    @location(7) point: vec2<f32>,
) -> VertexOutput {
    var out: VertexOutput;
    out.uv = vertex_uv;
    out.position = view.clip_from_world * vec4(vertex_position, 1.0);
    out.color = vertex_color;
    out.flags = flags;
    out.radius = radius;
    out.size = size;
    out.border = border;
    out.point = point;

    return out;
}

@group(1) @binding(0) var sprite_texture: texture_2d<f32>;
@group(1) @binding(1) var sprite_sampler: sampler;

// The returned value is the shortest distance from the given point to the boundary of the rounded 
// box.
// 
// Negative values indicate that the point is inside the rounded box, positive values that the point 
// is outside, and zero is exactly on the boundary.
//
// Arguments: 
//  - `point`        -> The function will return the distance from this point to the closest point on 
//                    the boundary.
//  - `size`         -> The maximum width and height of the box.
//  - `corner_radii` -> The radius of each rounded corner. Ordered counter clockwise starting 
//                    top left:
//                      x: top left, y: top right, z: bottom right, w: bottom left.
fn sd_rounded_box(point: vec2<f32>, size: vec2<f32>, corner_radii: vec4<f32>) -> f32 {
    // If 0.0 < y then select bottom left (w) and bottom right corner radius (z).
    // Else select top left (x) and top right corner radius (y).
    let rs = select(corner_radii.xy, corner_radii.wz, 0.0 < point.y);
    // w and z are swapped above so that both pairs are in left to right order, otherwise this second 
    // select statement would return the incorrect value for the bottom pair.
    let radius = select(rs.x, rs.y, 0.0 < point.x);
    // Vector from the corner closest to the point, to the point.
    let corner_to_point = abs(point) - 0.5 * size;
    // Vector from the center of the radius circle to the point.
    let q = corner_to_point + radius;
    // Length from center of the radius circle to the point, zeros a component if the point is not 
    // within the quadrant of the radius circle that is part of the curved corner.
    let l = length(max(q, vec2(0.0)));
    let m = min(max(q.x, q.y), 0.0);
    return l + m - radius;
}

fn sd_inset_rounded_box(point: vec2<f32>, size: vec2<f32>, radius: vec4<f32>, inset: vec4<f32>) -> f32 {
    let inner_size = size - inset.xy - inset.zw;
    let inner_center = inset.xy + 0.5 * inner_size - 0.5 * size;
    let inner_point = point - inner_center;

    var r = radius;

    // Top left corner.
    r.x = r.x - max(inset.x, inset.y);

    // Top right corner.
    r.y = r.y - max(inset.z, inset.y);

    // Bottom right corner.
    r.z = r.z - max(inset.z, inset.w); 

    // Bottom left corner.
    r.w = r.w - max(inset.x, inset.w);

    let half_size = inner_size * 0.5;
    let min_size = min(half_size.x, half_size.y);

    r = min(max(r, vec4(0.0)), vec4<f32>(min_size));

    return sd_rounded_box(inner_point, inner_size, r);
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
    radius: vec4<f32>,
    border: vec4<f32>,
    flags: u32,
) -> vec4<f32> {
    // Signed distances. The magnitude is the distance of the point from the edge of the shape.
    // * Negative values indicate that the point is inside the shape.
    // * Zero values indicate the point is on the edge of the shape.
    // * Positive values indicate the point is outside the shape.

    // Signed distance from the exterior boundary.
    let external_distance = sd_rounded_box(point, size, radius);

    // Signed distance from the border's internal edge (the signed distance is negative if the point 
    // is inside the rect but not on the border).
    // If the border size is set to zero, this is the same as the external distance.
    let internal_distance = sd_inset_rounded_box(point, size, radius, border);

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
    radius: vec4<f32>,
    border: vec4<f32>,
    flags: u32,
) -> vec4<f32> {
    // When drawing the background only draw the internal area and not the border.
    let internal_distance = sd_inset_rounded_box(point, size, radius, border) * select(1., -1, enabled(flags, INVERT));

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
        return draw_uinode_border(color, in.point, in.size, in.radius, in.border, in.flags);
    } else {
        return draw_uinode_background(color, in.point, in.size, in.radius, in.border, in.flags);
    }
}

#ifdef SUBPIXEL
// RGB subpixel text fragment path.
//
// The glyph atlas bound to `sprite_texture` stores *three per-channel
// coverage values* per pixel (one for each of the LCD stripe's R / G / B
// subpixels), produced by `swash`'s `Format::Subpixel` rasteriser in
// `bevy_text`. The sample is therefore not a colour — each channel is an
// alpha for its matching subpixel. We emit dual-source fragments so the
// hardware blender can consume per-channel alpha: `@blend_src(0)` carries
// the foreground colour and `@blend_src(1)` carries the per-channel alpha
// that the destination factor `OneMinusSrc1` multiplies against the
// existing framebuffer value.
//
// The contrast + gamma math is ported verbatim from Zed's GPUI subpixel
// shader, which in turn follows Skia's LCD text correction. Enhanced-
// contrast is luminance-adapted ("light-on-dark") so dark-mode text
// doesn't bloom; the gamma ratios are a lookup-driven adjustment around a
// target gamma (GPUI's default is 1.8).
//
// The tuning values (`enhanced_contrast`, `gamma_ratios`) are read from
// the `SubpixelSettings` uniform bound at `@group(0) @binding(1)`. The
// defaults (GPUI's `RenderingParameters::new()`) are baked into
// `SubpixelTextUniforms::default` in `bevy_ui_render::lib.rs`; phase-05
// plumbs a `SubpixelTextSettings` / `SubpixelLcdLayout` resource pair to
// override them at runtime.
struct SubpixelOutput {
    @location(0) @blend_src(0) color: vec4<f32>,
    @location(0) @blend_src(1) alpha_mask: vec4<f32>,
}

fn color_brightness(color: vec3<f32>) -> f32 {
    return dot(color, vec3<f32>(0.30, 0.59, 0.11));
}

fn light_on_dark_contrast(enhanced_contrast: f32, color: vec3<f32>) -> f32 {
    let brightness = color_brightness(color);
    let multiplier = saturate(4.0 * (0.75 - brightness));
    return enhanced_contrast * multiplier;
}

fn enhance_contrast3(alpha: vec3<f32>, k: f32) -> vec3<f32> {
    return alpha * (k + 1.0) / (alpha * k + 1.0);
}

fn apply_alpha_correction3(a: vec3<f32>, b: vec3<f32>, g: vec4<f32>) -> vec3<f32> {
    let brightness_adjustment = g.x * b + g.y;
    let correction = brightness_adjustment * a + (g.z * b + g.w);
    return a + a * (1.0 - a) * correction;
}

fn apply_contrast_and_gamma_correction3(
    sample: vec3<f32>,
    fg: vec3<f32>,
    enhanced_contrast_factor: f32,
    gamma_ratios: vec4<f32>,
) -> vec3<f32> {
    let enhanced_contrast = light_on_dark_contrast(enhanced_contrast_factor, fg);
    let contrasted = enhance_contrast3(sample, enhanced_contrast);
    return apply_alpha_correction3(contrasted, fg, gamma_ratios);
}

// Remap the atlas's three per-channel coverage values to match the target
// panel's physical subpixel arrangement (see `SubpixelLcdLayout` once
// introduced in phase-05).
//
// The swash rasteriser (`bevy_text::font_atlas::get_outlined_glyph_texture`)
// emits the atlas with *horizontal RGB* pre-offset baked in: texel
// `(x, y).r` is already the left subpixel's coverage at logical pixel
// `(x, y)`, `.g` the centre, `.b` the right. We therefore only need to
// swizzle — not resample at offset UVs — to support BGR panels.
//
// The vertical variants have no correct remap available from a
// horizontally-offset atlas. They currently swap R/B (mirroring the
// horizontal BGR/RGB distinction) so the setting visibly affects output,
// but the result is *not* a correct vertical-subpixel antialiasing; a
// follow-up spec will add vertical-subpixel rasterisation and re-wire
// these variants.
fn swizzle_subpixel_atlas(atlas_rgb: vec3<f32>, layout_flags: u32) -> vec3<f32> {
    if layout_flags == SUBPIXEL_LAYOUT_HORIZONTAL_BGR
        || layout_flags == SUBPIXEL_LAYOUT_VERTICAL_BGR
    {
        return atlas_rgb.bgr;
    }
    return atlas_rgb;
}

@fragment
fn fragment_subpixel(in: VertexOutput) -> SubpixelOutput {
    // Sample three per-channel alpha coverages from the RGB subpixel atlas,
    // then swizzle by the panel's subpixel layout. The swash rasteriser has
    // already baked in horizontal per-channel UV offsets, so a single sample
    // plus a swizzle is both correct and optimal for `HorizontalRgb` /
    // `HorizontalBgr`. See `swizzle_subpixel_atlas` for the vertical caveat.
    let atlas_rgb = textureSample(sprite_texture, sprite_sampler, in.uv).rgb;
    let sample_rgb = swizzle_subpixel_atlas(atlas_rgb, subpixel_settings.layout_flags);

    let enhanced_contrast = subpixel_settings.enhanced_contrast;
    let gamma_ratios = subpixel_settings.gamma_ratios;

    let alpha_corrected = apply_contrast_and_gamma_correction3(
        sample_rgb,
        in.color.rgb,
        enhanced_contrast,
        gamma_ratios,
    );

    // Matches GPUI's `fs_subpixel_sprite`. With pipeline blend state
    //   color: { src = Src1, dst = OneMinusSrc1 }
    // this yields:
    //   result.rgb = fg.rgb * (color.a * alpha_corrected)
    //              + dst.rgb * (1 - color.a * alpha_corrected)
    // i.e. per-channel coverage of the foreground over the existing pixel.
    var out: SubpixelOutput;
    out.color = vec4<f32>(in.color.rgb, 1.0);
    out.alpha_mask = vec4<f32>(in.color.a * alpha_corrected, 1.0);
    return out;
}
#endif
