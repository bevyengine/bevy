#define_import_path bevy_core_pipeline::tonemapping


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

fn tonemap_curve3(v: vec3<f32>) -> vec3<f32> {
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
    let tm1 = tonemap_curve3(desat_col);

    boring_color = mix(tm0, tm1, bt * bt);

    return boring_color * final_mult;
}

// ------------------------------------------
// ------------- Tony McMapface -------------
// ------------------------------------------
// By Tomasz Stachowiak
// https://github.com/h3r2tic/tony-mc-mapface

const TONY_MC_MAPFACE_LUT_EV_RANGE = vec2<f32>(-13.0, 8.0);
const TONY_MC_MAPFACE_LUT_DIMS: f32 = 48.0;

fn tony_mc_mapface_lut_range_encode(x: vec3<f32>) -> vec3<f32> {
    return x / (x + 1.0);
}

fn sample_tony_mc_mapface_lut(stimulus: vec3<f32>) -> vec3<f32> {
    let range = tony_mc_mapface_lut_range_encode(exp2(TONY_MC_MAPFACE_LUT_EV_RANGE.xyy)).xy;
    let normalized = (tony_mc_mapface_lut_range_encode(stimulus) - range.x) / (range.y - range.x);
    var uv = saturate(normalized * (f32(TONY_MC_MAPFACE_LUT_DIMS - 1.0) / f32(TONY_MC_MAPFACE_LUT_DIMS)) + 0.5 / f32(TONY_MC_MAPFACE_LUT_DIMS));
    return sample_current_lut(uv).rgb;
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

// pow() but safe for NaNs/negatives
fn powsafe(color: vec3<f32>, power: f32) -> vec3<f32> {
    return pow(abs(color), vec3(power)) * sign(color);
}

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
fn convertOpenDomainToNormalizedLog2(color: vec3<f32>, minimum_ev: f32, maximum_ev: f32) -> vec3<f32> {
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

    prepared_image = convertOpenDomainToNormalizedLog2(prepared_image, -10.0, 6.5);
    
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
    let normalized = saturate(convertOpenDomainToNormalizedLog2(stimulus, -11.0, 12.0));
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

fn tone_mapping(in: vec4<f32>) -> vec4<f32> {
    var color = max(in.rgb, vec3(0.0));

    // Possible future grading:

    // highlight gain gamma: 0..
    // let luma = powsafe(vec3(tonemapping_luminance(color)), 1.0); 

    // highlight gain: 0.. 
    // color += color * luma.xxx * 1.0; 

    // Linear pre tonemapping grading
    color = saturation(color, view.color_grading.pre_saturation);
    color = powsafe(color, view.color_grading.gamma);
    color = color * powsafe(vec3(2.0), view.color_grading.exposure);
    color = max(color, vec3(0.0));

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
    color = saturation(color, view.color_grading.post_saturation);
    
    return vec4(color, in.a);
}

