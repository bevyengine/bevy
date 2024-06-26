#define_import_path bevy_core_pipeline::tonemapping

#import bevy_render::{
    view::ColorGrading,
    color_operations::{hsv_to_rgb, rgb_to_hsv},
    maths::{PI_2, powsafe},
}

#import bevy_core_pipeline::tonemapping_lut_bindings::{
    dt_lut_texture,
    dt_lut_sampler,
}

// Half the size of the crossfade region between shadows and midtones and
// between midtones and highlights. This value, 0.1, corresponds to 10% of the
// gamut on either side of the cutoff point.
const LEVEL_MARGIN: f32 = 0.1;

// The inverse reciprocal of twice the above, used when scaling the midtone
// region.
const LEVEL_MARGIN_DIV: f32 = 0.5 / LEVEL_MARGIN;

fn sample_current_lut(p: vec3<f32>) -> vec3<f32> {
    // Don't include code that will try to sample from LUTs if tonemap method doesn't require it
    // Allows this file to be imported without necessarily needing the lut texture bindings
#ifdef TONEMAP_METHOD_AGX
    return textureSampleLevel(dt_lut_texture, dt_lut_sampler, p, 0.0).rgb;
#else ifdef TONEMAP_METHOD_TONY_MC_MAPFACE
    return textureSampleLevel(dt_lut_texture, dt_lut_sampler, p, 0.0).rgb;
#else ifdef TONEMAP_METHOD_BLENDER_FILMIC
    return textureSampleLevel(dt_lut_texture, dt_lut_sampler, p, 0.0).rgb;
#else
    return vec3(1.0, 0.0, 1.0);
 #endif
}

// --------------------------------------
// --- SomewhatBoringDisplayTransform ---
// --------------------------------------
// By Tomasz Stachowiak

fn rgb_to_ycbcr(col: vec3<f32>) -> vec3<f32> {
    let m = mat3x3<f32>(
        0.2126, 0.7152, 0.0722,
        -0.1146, -0.3854, 0.5,
        0.5, -0.4542, -0.0458
    );
    return col * m;
}

fn ycbcr_to_rgb(col: vec3<f32>) -> vec3<f32> {
    let m = mat3x3<f32>(
        1.0, 0.0, 1.5748,
        1.0, -0.1873, -0.4681,
        1.0, 1.8556, 0.0
    );
    return max(vec3(0.0), col * m);
}

fn tonemap_curve(v: f32) -> f32 {
#ifdef 0
    // Large linear part in the lows, but compresses highs.
    float c = v + v * v + 0.5 * v * v * v;
    return c / (1.0 + c);
#else
    return 1.0 - exp(-v);
#endif
}

fn tonemap_curve3_(v: vec3<f32>) -> vec3<f32> {
    return vec3(tonemap_curve(v.r), tonemap_curve(v.g), tonemap_curve(v.b));
}

fn somewhat_boring_display_transform(col: vec3<f32>) -> vec3<f32> {
    var boring_color = col;
    let ycbcr = rgb_to_ycbcr(boring_color);

    let bt = tonemap_curve(length(ycbcr.yz) * 2.4);
    var desat = max((bt - 0.7) * 0.8, 0.0);
    desat *= desat;

    let desat_col = mix(boring_color.rgb, ycbcr.xxx, desat);

    let tm_luma = tonemap_curve(ycbcr.x);
    let tm0 = boring_color.rgb * max(0.0, tm_luma / max(1e-5, tonemapping_luminance(boring_color.rgb)));
    let final_mult = 0.97;
    let tm1 = tonemap_curve3_(desat_col);

    boring_color = mix(tm0, tm1, bt * bt);

    return boring_color * final_mult;
}

// ------------------------------------------
// ------------- Tony McMapface -------------
// ------------------------------------------
// By Tomasz Stachowiak
// https://github.com/h3r2tic/tony-mc-mapface

const TONY_MC_MAPFACE_LUT_DIMS: f32 = 48.0;

fn sample_tony_mc_mapface_lut(stimulus: vec3<f32>) -> vec3<f32> {
    var uv = (stimulus / (stimulus + 1.0)) * (f32(TONY_MC_MAPFACE_LUT_DIMS - 1.0) / f32(TONY_MC_MAPFACE_LUT_DIMS)) + 0.5 / f32(TONY_MC_MAPFACE_LUT_DIMS);
    return sample_current_lut(saturate(uv)).rgb;
}

// ---------------------------------
// ---------- ACES Fitted ----------
// ---------------------------------

// Same base implementation that Godot 4.0 uses for Tonemap ACES.

// https://github.com/TheRealMJP/BakingLab/blob/master/BakingLab/ACES.hlsl

// The code in this file was originally written by Stephen Hill (@self_shadow), who deserves all
// credit for coming up with this fit and implementing it. Buy him a beer next time you see him. :)

fn RRTAndODTFit(v: vec3<f32>) -> vec3<f32> {
    let a = v * (v + 0.0245786) - 0.000090537;
    let b = v * (0.983729 * v + 0.4329510) + 0.238081;
    return a / b;
}

fn ACESFitted(color: vec3<f32>) -> vec3<f32> {
    var fitted_color = color;

    // sRGB => XYZ => D65_2_D60 => AP1 => RRT_SAT
    let rgb_to_rrt = mat3x3<f32>(
        vec3(0.59719, 0.35458, 0.04823),
        vec3(0.07600, 0.90834, 0.01566),
        vec3(0.02840, 0.13383, 0.83777)
    );

    // ODT_SAT => XYZ => D60_2_D65 => sRGB
    let odt_to_rgb = mat3x3<f32>(
        vec3(1.60475, -0.53108, -0.07367),
        vec3(-0.10208, 1.10813, -0.00605),
        vec3(-0.00327, -0.07276, 1.07602)
    );

    fitted_color *= rgb_to_rrt;

    // Apply RRT and ODT
    fitted_color = RRTAndODTFit(fitted_color);

    fitted_color *= odt_to_rgb;

    // Clamp to [0, 1]
    fitted_color = saturate(fitted_color);

    return fitted_color;
}

// -------------------------------
// ------------- AgX -------------
// -------------------------------
// By Troy Sobotka
// https://github.com/MrLixm/AgXc
// https://github.com/sobotka/AgX

/*
    Increase color saturation of the given color data.
    :param color: expected sRGB primaries input
    :param saturationAmount: expected 0-1 range with 1=neutral, 0=no saturation.
    -- ref[2] [4]
*/
fn saturation(color: vec3<f32>, saturationAmount: f32) -> vec3<f32> {
    let luma = tonemapping_luminance(color);
    return mix(vec3(luma), color, vec3(saturationAmount));
}

/*
    Output log domain encoded data.
    Similar to OCIO lg2 AllocationTransform.
    ref[0]
*/
fn convertOpenDomainToNormalizedLog2_(color: vec3<f32>, minimum_ev: f32, maximum_ev: f32) -> vec3<f32> {
    let in_midgray = 0.18;

    // remove negative before log transform
    var normalized_color = max(vec3(0.0), color);
    // avoid infinite issue with log -- ref[1]
    normalized_color = select(normalized_color, 0.00001525878 + normalized_color, normalized_color  < vec3<f32>(0.00003051757));
    normalized_color = clamp(
        log2(normalized_color / in_midgray),
        vec3(minimum_ev),
        vec3(maximum_ev)
    );
    let total_exposure = maximum_ev - minimum_ev;

    return (normalized_color - minimum_ev) / total_exposure;
}

// Inverse of above
fn convertNormalizedLog2ToOpenDomain(color: vec3<f32>, minimum_ev: f32, maximum_ev: f32) -> vec3<f32> {
    var open_color = color;
    let in_midgray = 0.18;
    let total_exposure = maximum_ev - minimum_ev;

    open_color = (open_color * total_exposure) + minimum_ev;
    open_color = pow(vec3(2.0), open_color);
    open_color = open_color * in_midgray;

    return open_color;
}


/*=================
    Main processes
=================*/

// Prepare the data for display encoding. Converted to log domain.
fn applyAgXLog(Image: vec3<f32>) -> vec3<f32> {
    var prepared_image = max(vec3(0.0), Image); // clamp negatives
    let r = dot(prepared_image, vec3(0.84247906, 0.0784336, 0.07922375));
    let g = dot(prepared_image, vec3(0.04232824, 0.87846864, 0.07916613));
    let b = dot(prepared_image, vec3(0.04237565, 0.0784336, 0.87914297));
    prepared_image = vec3(r, g, b);

    prepared_image = convertOpenDomainToNormalizedLog2_(prepared_image, -10.0, 6.5);

    prepared_image = clamp(prepared_image, vec3(0.0), vec3(1.0));
    return prepared_image;
}

fn applyLUT3D(Image: vec3<f32>, block_size: f32) -> vec3<f32> {
    return sample_current_lut(Image * ((block_size - 1.0) / block_size) + 0.5 / block_size).rgb;
}

// -------------------------
// -------------------------
// -------------------------

fn sample_blender_filmic_lut(stimulus: vec3<f32>) -> vec3<f32> {
    let block_size = 64.0;
    let normalized = saturate(convertOpenDomainToNormalizedLog2_(stimulus, -11.0, 12.0));
    return applyLUT3D(normalized, block_size);
}

// from https://64.github.io/tonemapping/
// reinhard on RGB oversaturates colors
fn tonemapping_reinhard(color: vec3<f32>) -> vec3<f32> {
    return color / (1.0 + color);
}

fn tonemapping_reinhard_extended(color: vec3<f32>, max_white: f32) -> vec3<f32> {
    let numerator = color * (1.0 + (color / vec3<f32>(max_white * max_white)));
    return numerator / (1.0 + color);
}

// luminance coefficients from Rec. 709.
// https://en.wikipedia.org/wiki/Rec._709
fn tonemapping_luminance(v: vec3<f32>) -> f32 {
    return dot(v, vec3<f32>(0.2126, 0.7152, 0.0722));
}

fn tonemapping_change_luminance(c_in: vec3<f32>, l_out: f32) -> vec3<f32> {
    let l_in = tonemapping_luminance(c_in);
    return c_in * (l_out / l_in);
}

fn tonemapping_reinhard_luminance(color: vec3<f32>) -> vec3<f32> {
    let l_old = tonemapping_luminance(color);
    let l_new = l_old / (1.0 + l_old);
    return tonemapping_change_luminance(color, l_new);
}

fn rgb_to_srgb_simple(color: vec3<f32>) -> vec3<f32> {
    return pow(color, vec3<f32>(1.0 / 2.2));
}

// Source: Advanced VR Rendering, GDC 2015, Alex Vlachos, Valve, Slide 49
// https://media.steampowered.com/apps/valve/2015/Alex_Vlachos_Advanced_VR_Rendering_GDC2015.pdf
fn screen_space_dither(frag_coord: vec2<f32>) -> vec3<f32> {
    var dither = vec3<f32>(dot(vec2<f32>(171.0, 231.0), frag_coord)).xxx;
    dither = fract(dither.rgb / vec3<f32>(103.0, 71.0, 97.0));
    return (dither - 0.5) / 255.0;
}

// Performs the "sectional" color grading: i.e. the color grading that applies
// individually to shadows, midtones, and highlights.
fn sectional_color_grading(
    in: vec3<f32>,
    color_grading: ptr<function, ColorGrading>,
) -> vec3<f32> {
    var color = in;

    // Determine whether the color is a shadow, midtone, or highlight. Colors
    // close to the edges are considered a mix of both, to avoid sharp
    // discontinuities. The formulas are taken from Blender's compositor.

    let level = (color.r + color.g + color.b) / 3.0;

    // Determine whether this color is a shadow, midtone, or highlight. If close
    // to the cutoff points, blend between the two to avoid sharp color
    // discontinuities.
    var levels = vec3(0.0);
    let midtone_range = (*color_grading).midtone_range;
    if (level < midtone_range.x - LEVEL_MARGIN) {
        levels.x = 1.0;
    } else if (level < midtone_range.x + LEVEL_MARGIN) {
        levels.y = ((level - midtone_range.x) * LEVEL_MARGIN_DIV) + 0.5;
        levels.z = 1.0 - levels.y;
    } else if (level < midtone_range.y - LEVEL_MARGIN) {
        levels.y = 1.0;
    } else if (level < midtone_range.y + LEVEL_MARGIN) {
        levels.z = ((level - midtone_range.y) * LEVEL_MARGIN_DIV) + 0.5;
        levels.y = 1.0 - levels.z;
    } else {
        levels.z = 1.0;
    }

    // Calculate contrast/saturation/gamma/gain/lift.
    let contrast = dot(levels, (*color_grading).contrast);
    let saturation = dot(levels, (*color_grading).saturation);
    let gamma = dot(levels, (*color_grading).gamma);
    let gain = dot(levels, (*color_grading).gain);
    let lift = dot(levels, (*color_grading).lift);

    // Adjust saturation and contrast.
    let luma = tonemapping_luminance(color);
    color = luma + saturation * (color - luma);
    color = 0.5 + (color - 0.5) * contrast;

    // The [ASC CDL] formula for color correction. Given *i*, an input color, we
    // have:
    //
    //     out = (i × s + o)ⁿ
    //
    // Following the normal photographic naming convention, *gain* is the *s*
    // factor, *lift* is the *o* term, and the inverse of *gamma* is the *n*
    // exponent.
    //
    // [ASC CDL]: https://en.wikipedia.org/wiki/ASC_CDL#Combined_Function
    color = powsafe(color * gain + lift, 1.0 / gamma);

    // Account for exposure.
    color = color * powsafe(vec3(2.0), (*color_grading).exposure);
    return max(color, vec3(0.0));
}

fn tone_mapping(in: vec4<f32>, in_color_grading: ColorGrading) -> vec4<f32> {
    var color = max(in.rgb, vec3(0.0));
    var color_grading = in_color_grading;   // So we can take pointers to it.

    // Rotate hue if needed, by converting to and from HSV. Remember that hue is
    // an angle, so it needs to be modulo 2π.
#ifdef HUE_ROTATE
    var hsv = rgb_to_hsv(color);
    hsv.r = (hsv.r + color_grading.hue) % PI_2;
    color = hsv_to_rgb(hsv);
#endif

    // Perform white balance correction. Conveniently, this is a linear
    // transform. The matrix was pre-calculated from the temperature and tint
    // values on the CPU.
#ifdef WHITE_BALANCE
    color = max(color_grading.balance * color, vec3(0.0));
#endif

    // Perform the "sectional" color grading: i.e. the color grading that
    // applies individually to shadows, midtones, and highlights.
#ifdef SECTIONAL_COLOR_GRADING
    color = sectional_color_grading(color, &color_grading);
#else
    // If we're not doing sectional color grading, the exposure might still need
    // to be applied, for example when using auto exposure.
    color = color * powsafe(vec3(2.0), color_grading.exposure);
#endif

    // tone_mapping
#ifdef TONEMAP_METHOD_NONE
    color = color;
#else ifdef TONEMAP_METHOD_REINHARD
    color = tonemapping_reinhard(color.rgb);
#else ifdef TONEMAP_METHOD_REINHARD_LUMINANCE
    color = tonemapping_reinhard_luminance(color.rgb);
#else ifdef TONEMAP_METHOD_ACES_FITTED
    color = ACESFitted(color.rgb);
#else ifdef TONEMAP_METHOD_AGX
    color = applyAgXLog(color);
    color = applyLUT3D(color, 32.0);
#else ifdef TONEMAP_METHOD_SOMEWHAT_BORING_DISPLAY_TRANSFORM
    color = somewhat_boring_display_transform(color.rgb);
#else ifdef TONEMAP_METHOD_TONY_MC_MAPFACE
    color = sample_tony_mc_mapface_lut(color);
#else ifdef TONEMAP_METHOD_BLENDER_FILMIC
    color = sample_blender_filmic_lut(color.rgb);
#endif

    // Perceptual post tonemapping grading
    color = saturation(color, color_grading.post_saturation);

    return vec4(color, in.a);
}

// This is an **incredibly crude** approximation of the inverse of the tone mapping function.
// We assume here that there's a simple linear relationship between the input and output
// which is not true at all, but useful to at least preserve the overall luminance of colors
// when sampling from an already tonemapped image. (e.g. for transmissive materials when HDR is off)
fn approximate_inverse_tone_mapping(in: vec4<f32>, color_grading: ColorGrading) -> vec4<f32> {
    let out = tone_mapping(in, color_grading);
    let approximate_ratio = length(in.rgb) / length(out.rgb);
    return vec4(in.rgb * approximate_ratio, in.a);
}
