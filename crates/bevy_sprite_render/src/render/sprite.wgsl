// `enable dual_source_blending;` must appear before any other global
// declaration per the WGSL spec. We keep it unconditional (rather than
// gating it on `#ifdef SUBPIXEL`) because naga_oil's preprocessor expands
// `#define_import_path` / `#import` as leading globals, which makes a
// preprocessor-wrapped `enable` land "after" them and trips a parse error.
// The directive is harmless when the SUBPIXEL path is inactive — naga only
// requires the corresponding DSB capability at compile time on pipelines
// that actually use `@blend_src`. Mirrors the identical workaround in
// `crates/bevy_ui_render/src/ui.wgsl` (and the precedent in
// `crates/bevy_pbr/src/atmosphere/render_sky.wgsl`).
enable dual_source_blending;

#ifdef TONEMAP_IN_SHADER
#import bevy_core_pipeline::tonemapping
#endif
#ifdef SRGB_OUTPUT
#import bevy_render::color_operations::linear_to_srgb
#endif
#ifdef OKLAB_OUTPUT
#import bevy_render::color_operations::linear_rgb_to_oklab
#endif

#import bevy_render::{
    maths::affine3_to_square,
    view::View,
}

#import bevy_sprite::sprite_view_bindings::view

// Subpixel text tuning uniform. Populated from `SubpixelTextSettings` +
// `SubpixelLcdLayout` (both in `bevy_text`) at prepare time by
// `prepare_sprite_view_bind_groups`. Bound at `@group(0) @binding(3)` on
// every sprite pipeline variant so the bind-group layout is shared between
// the standard sprite fragment entry and `fragment_subpixel` below.
//
// Layout mirrors the Rust `SubpixelTextUniforms` struct byte-for-byte:
// `enhanced_contrast` (4 bytes) + `layout_flags` (4 bytes) + `_pad` (8
// bytes) fills the leading 16-byte slot so `gamma_ratios` lands on its
// natural 16-byte boundary. Total 32 bytes.
//
// `layout_flags` discriminant (keep in sync with
// `SubpixelLcdLayout::pack_u32` in `bevy_text/src/subpixel.rs` and the
// `SUBPIXEL_LAYOUT_*` constants in `bevy_ui_render/src/ui.wgsl`):
//   0 = HorizontalRgb  (atlas R, G, B in that order — identity swizzle)
//   1 = HorizontalBgr  (swap R/B — text on a BGR panel)
struct SubpixelSettings {
    enhanced_contrast: f32,
    layout_flags: u32,
    _pad: vec2<f32>,
    gamma_ratios: vec4<f32>,
}
@group(0) @binding(3) var<uniform> subpixel_settings: SubpixelSettings;

const SUBPIXEL_LAYOUT_HORIZONTAL_RGB: u32 = 0u;
const SUBPIXEL_LAYOUT_HORIZONTAL_BGR: u32 = 1u;

struct VertexInput {
    @builtin(vertex_index) index: u32,
    // NOTE: Instance-rate vertex buffer members prefixed with i_
    // NOTE: i_model_transpose_colN are the 3 columns of a 3x4 matrix that is the transpose of the
    // affine 4x3 model matrix.
    @location(0) i_model_transpose_col0: vec4<f32>,
    @location(1) i_model_transpose_col1: vec4<f32>,
    @location(2) i_model_transpose_col2: vec4<f32>,
    @location(3) i_color: vec4<f32>,
    @location(4) i_uv_offset_scale: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) @interpolate(flat) color: vec4<f32>,
};

@vertex
fn vertex(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    let vertex_position = vec3<f32>(
        f32(in.index & 0x1u),
        f32((in.index & 0x2u) >> 1u),
        0.0
    );

    out.clip_position = view.clip_from_world * affine3_to_square(mat3x4<f32>(
        in.i_model_transpose_col0,
        in.i_model_transpose_col1,
        in.i_model_transpose_col2,
    )) * vec4<f32>(vertex_position, 1.0);
    out.uv = vec2<f32>(vertex_position.xy) * in.i_uv_offset_scale.zw + in.i_uv_offset_scale.xy;
    out.color = in.i_color;

    return out;
}

@group(1) @binding(0) var sprite_texture: texture_2d<f32>;
@group(1) @binding(1) var sprite_sampler: sampler;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    var color = in.color * textureSample(sprite_texture, sprite_sampler, in.uv);

#ifdef TONEMAP_IN_SHADER
    color = tonemapping::tone_mapping(color, view.color_grading);
#endif

#ifdef SRGB_OUTPUT
    color = vec4(linear_to_srgb(color.rgb), color.a);
#endif

#ifdef OKLAB_OUTPUT
    color = vec4(linear_rgb_to_oklab(color.rgb), color.a);
#endif

    return color;
}

#ifdef SUBPIXEL
// RGB subpixel text fragment path — sprite-render edition.
//
// This block is intentionally a verbatim port of `fragment_subpixel` in
// `crates/bevy_ui_render/src/ui.wgsl` (added in phase-04 of spec/0013).
// Cross-crate WGSL sharing in Bevy goes through `#define_import_path` /
// `#import`, but both `ui.wgsl` and this file are top-level pipeline
// shaders that also declare their own `@fragment` entries — making them
// imports of each other creates a cycle. ~60 lines of duplication is the
// practical trade-off; if either side changes materially the other must
// be kept in sync.
//
// The glyph atlas bound to `sprite_texture` stores *three per-channel
// coverage values* per pixel (one for each of the LCD stripe's R / G / B
// subpixels), produced by `swash`'s `Format::Subpixel` rasteriser in
// `bevy_text`. The sample is therefore not a color — each channel is an
// alpha for its matching subpixel. We emit dual-source fragments so the
// hardware blender can consume per-channel alpha: `@blend_src(0)` carries
// the foreground color, `@blend_src(1)` carries the per-channel alpha
// that the destination factor `OneMinusSrc1` multiplies against the
// existing framebuffer value.
//
// The contrast + gamma math is ported verbatim from Zed's GPUI subpixel
// shader, which in turn follows Skia's LCD text correction; see the
// comment on the UI side for the full derivation.
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
// panel's physical subpixel arrangement. Matches `swizzle_subpixel_atlas`
// in `ui.wgsl`; see that function's comment for the BGR caveat.
fn swizzle_subpixel_atlas(atlas_rgb: vec3<f32>, layout_flags: u32) -> vec3<f32> {
    if layout_flags == SUBPIXEL_LAYOUT_HORIZONTAL_BGR {
        return atlas_rgb.bgr;
    }
    return atlas_rgb;
}

@fragment
fn fragment_subpixel(in: VertexOutput) -> SubpixelOutput {
    // Sample three per-channel alpha coverages from the RGB subpixel atlas,
    // then swizzle by the panel's subpixel layout. The swash rasteriser
    // has already baked in horizontal per-channel UV offsets, so a single
    // sample plus a swizzle is both correct and optimal for
    // `HorizontalRgb` / `HorizontalBgr`.
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

    // Matches GPUI's `fs_subpixel_sprite` and the UI crate's
    // `fragment_subpixel`. With pipeline blend state
    // `color: { src = Src1, dst = OneMinusSrc1 }` this yields:
    //   result.rgb = fg.rgb * (color.a * alpha_corrected)
    //              + dst.rgb * (1 - color.a * alpha_corrected)
    // i.e. per-channel coverage of the foreground over the existing pixel.
    var out: SubpixelOutput;
    out.color = vec4<f32>(in.color.rgb, 1.0);
    out.alpha_mask = vec4<f32>(in.color.a * alpha_corrected, 1.0);
    return out;
}
#endif
