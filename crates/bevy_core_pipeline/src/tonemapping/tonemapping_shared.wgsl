#define_import_path bevy_core_pipeline::tonemapping

// -------------------------------------
// ------------- SBDT 1 ----------------
// -------------------------------------
// Rec. 709
fn srgb_to_luminance(col: vec3<f32>) -> f32 {
    return dot(vec3(0.2126, 0.7152, 0.0722), col);
}

fn rgb_to_ycbcr(col: vec3<f32>) -> vec3<f32> {
	let m = mat3x3<f32>(0.2126, 0.7152, 0.0722, -0.1146,-0.3854, 0.5, 0.5,-0.4542,-0.0458);
    return col * m;
}

fn ycbcr_to_rgb(col: vec3<f32>) -> vec3<f32> {
	let m = mat3x3<f32>(1.0, 0.0, 1.5748, 1.0, -0.1873, -.4681, 1.0, 1.8556, 0.0);
    return max(vec3(0.0), col * m);
}

fn tonemap_curve(v: f32) -> f32 {
    #ifdef 0
    // Large linear part in the lows, but compresses highs.
    float c = v + v*v + 0.5*v*v*v;
    return c / (1.0 + c);
    #else
    return 1.0 - exp(-v);
    #endif
}

fn tonemap_curve3(v: vec3<f32>) -> vec3<f32> {
    return vec3(tonemap_curve(v.r), tonemap_curve(v.g), tonemap_curve(v.b));
}

fn tonemapping_sbdt(col: vec3<f32>) -> vec3<f32> {
    var col = col;
    let ycbcr = rgb_to_ycbcr(col);

    let bt = tonemap_curve(length(ycbcr.yz) * 2.4);
    var desat = max((bt - 0.7) * 0.8, 0.0);
    desat *= desat;

    let desat_col = mix(col.rgb, ycbcr.xxx, desat);

    let tm_luma = tonemap_curve(ycbcr.x);
    let tm0 = col.rgb * max(0.0, tm_luma / max(1e-5, srgb_to_luminance(col.rgb)));
    let final_mult = 0.97;
    let tm1 = tonemap_curve3(desat_col);

    col = mix(tm0, tm1, bt * bt);

    return col * final_mult;
}

// -------------------------------------
// ------------- SBDT 2 ----------------
// -------------------------------------

const SBDT2_LUT_EV_RANGE = vec2<f32>(-13.0, 8.0);
const SBDT2_LUT_DIMS: f32 = 48.0;

fn sbdt2_lut_range_encode(x: vec3<f32>) -> vec3<f32> {
    return x / (x + 1.0);
}

fn sample_sbdt2_lut(stimulus: vec3<f32>) -> vec3<f32> {
    let range = sbdt2_lut_range_encode(exp2(SBDT2_LUT_EV_RANGE.xyy)).xy;
    let normalized = (sbdt2_lut_range_encode(stimulus) - range.x) / (range.y - range.x);
    var uv = saturate(normalized * (f32(SBDT2_LUT_DIMS - 1.0) / f32(SBDT2_LUT_DIMS)) + 0.5 / f32(SBDT2_LUT_DIMS));
    uv.y = 1.0 - uv.y;
    return textureSampleLevel(dt_lut_texture, dt_lut_sampler, uv, 0.0).rgb;
}

// -------------------------
// ---- aces from godot ----
// -------------------------
// Adapted from https://github.com/TheRealMJP/BakingLab/blob/master/BakingLab/ACES.hlsl
// (MIT License).
fn tonemapping_aces_godot_4(color: vec3<f32>, white: f32) -> vec3<f32> {
    var color = color;
    var white = white;

    // TODO make const
	let exposure_bias = 1.8;
	let A = 0.0245786;
	let B = 0.000090537;
	let C = 0.983729;
	let D = 0.432951;
	let E = 0.238081;

	// Exposure bias baked into transform to save shader instructions. Equivalent to `color *= exposure_bias`
	let rgb_to_rrt = mat3x3<f32>(
			vec3(0.59719f * exposure_bias, 0.35458f * exposure_bias, 0.04823f * exposure_bias),
			vec3(0.07600f * exposure_bias, 0.90834f * exposure_bias, 0.01566f * exposure_bias),
			vec3(0.02840f * exposure_bias, 0.13383f * exposure_bias, 0.83777f * exposure_bias));

	let odt_to_rgb = mat3x3<f32>(
			vec3(1.60475f, -0.53108f, -0.07367f),
			vec3(-0.10208f, 1.10813f, -0.00605f),
			vec3(-0.00327f, -0.07276f, 1.07602f));

	color *= rgb_to_rrt;
	var color_tonemapped = (color * (color + A) - B) / (color * (C * color + D) + E);
	color_tonemapped *= odt_to_rgb;

	white *= exposure_bias;
	let white_tonemapped = (white * (white + A) - B) / (white * (C * white + D) + E);

	return color_tonemapped / white_tonemapped;
}

// --------------------------------
// ---- tonemap_filmic godot 4 ----
// --------------------------------
fn tonemap_filmic_godot_4(color: vec3<f32>, white: f32) -> vec3<f32> {
	// exposure bias: input scale (color *= bias, white *= bias) to make the brightness consistent with other tonemappers
	// also useful to scale the input to the range that the tonemapper is designed for (some require very high input values)
	// has no effect on the curve's general shape or visual properties
    // TODO make const
	let exposure_bias = 2.0;
	let A = 0.22 * exposure_bias * exposure_bias; // bias baked into constants for performance
	let B = 0.30 * exposure_bias;
	let C = 0.10;
	let D = 0.20;
	let E = 0.01;
	let F = 0.30;

	let color_tonemapped = ((color * (A * color + C * B) + D * E) / (color * (A * color + B) + D * F)) - E / F;
	let white_tonemapped = ((white * (A * white + C * B) + D * E) / (white * (A * white + B) + D * F)) - E / F;

	return color_tonemapped / white_tonemapped;
}

// --------------------------------
// ------------- AgX --------------
// --------------------------------
// https://github.com/MrLixm/AgXc
// https://github.com/sobotka/AgX

// pow() but safe for NaNs/negatives
fn powsafe(color: vec3<f32>, power: f32) -> vec3<f32> {
    return pow(abs(color), vec3(power)) * sign(color);
}

/*
    Increase color saturation of the given color data.
    :param color: expected sRGB primaries input
    :oaram saturationAmount: expected 0-1 range with 1=neutral, 0=no saturation.
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
    let in_midgrey = 0.18;

    // remove negative before log transform
    var color = max(vec3(0.0), color);
    // avoid infinite issue with log -- ref[1]
    color = select(color, 0.00001525878 + color, color  < 0.00003051757);
    color = clamp(
        log2(color / in_midgrey),
        vec3(minimum_ev),
        vec3(maximum_ev)
    );
    let total_exposure = maximum_ev - minimum_ev;

    return (color - minimum_ev) / total_exposure;
}

/*=================
    Main processes
=================*/

// Prepare the data for display encoding. Converted to log domain.
fn applyAgXLog(Image: vec3<f32>) -> vec3<f32> {
    var Image = max(vec3(0.0), Image); // clamp negatives
	let r = dot(Image, vec3(0.84247906, 0.0784336, 0.07922375));
	let g = dot(Image, vec3(0.04232824, 0.87846864, 0.07916613));
	let b = dot(Image, vec3(0.04237565, 0.0784336, 0.87914297));
	Image = vec3(r, g, b);

    Image = convertOpenDomainToNormalizedLog2(Image, -10.0, 6.5);
    
    Image = clamp(Image, vec3(0.0), vec3(1.0));
    return Image;
}

fn applyLUT3D(Image: vec3<f32>, block_size: f32, dimensions: vec2<f32>, offset: vec2<f32>) -> vec3<f32> {
    return textureSampleLevel(dt_lut_texture, dt_lut_sampler, Image * ((block_size - 1.0) / block_size) + 0.5 / block_size, 0.0).rgb;
}

// -------------------------
// -------------------------
// -------------------------

fn rgb_to_srgb_simple(color: vec3<f32>) -> vec3<f32> {
    return pow(color, vec3<f32>(1.0 / 2.2));
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

// Source: Advanced VR Rendering, GDC 2015, Alex Vlachos, Valve, Slide 49
// https://media.steampowered.com/apps/valve/2015/Alex_Vlachos_Advanced_VR_Rendering_GDC2015.pdf
fn screen_space_dither(frag_coord: vec2<f32>) -> vec3<f32> {
    var dither = vec3<f32>(dot(vec2<f32>(171.0, 231.0), frag_coord)).xxx;
    dither = fract(dither.rgb / vec3<f32>(103.0, 71.0, 97.0));
    return (dither - 0.5) / 255.0;
}

fn tone_mapping(in: vec4<f32>) -> vec4<f32> {
    var color = max(in.rgb, vec3(0.0));

    //let luma = powsafe(vec3(tonemapping_luminance(color)), 1.0); // highlight gain gamma: 0..
    //color += color * luma.xxx * 1.0; // highlight gain: 0.. 

    color = saturation(color, view.color_grading.pre_saturation);
    color = powsafe(color, view.color_grading.gamma);
    color = color * powsafe(vec3(2.0), view.color_grading.exposure);
    color = max(color, vec3(0.0));

    // tone_mapping
#ifdef TONEMAP_METHOD_NONE
    color = color;
#endif
#ifdef TONEMAP_METHOD_REINHARD
    color = tonemapping_reinhard(color.rgb);
#endif
#ifdef TONEMAP_METHOD_REINHARD_LUMINANCE
    color = tonemapping_reinhard_luminance(color.rgb);
#endif
#ifdef TONEMAP_METHOD_ACES
    // TODO figure out correct value for white here, or factor it out
    color = tonemapping_aces_godot_4(color.rgb, 1000.0);
#endif
#ifdef TONEMAP_METHOD_AGX
    color = applyAgXLog(color);
    color = applyLUT3D(color, 32.0, vec2<f32>(1024.0, 32.0), vec2(0.0));
#endif
#ifdef TONEMAP_METHOD_SBDT
    color = tonemapping_sbdt(color.rgb);
#endif
#ifdef TONEMAP_METHOD_SBDT2
    color = sample_sbdt2_lut(color);
#endif
#ifdef TONEMAP_METHOD_BLENDER_FILMIC
    let block_size = 64.0;
    let selector = 0.0;
    var c = color.rgb; // * 0.82 somewhat matches tonemapping_reinhard_luminance
    c = convertOpenDomainToNormalizedLog2(c, -11.0, 12.0);
    c = saturate(c);
    c = applyLUT3D(c, block_size, vec2<f32>(4096.0, 64.0), vec2(0.0, 32.0 + block_size * selector));
    color = c;
#endif

    color = saturation(color, view.color_grading.post_saturation);

    // Gamma correction.
    // Not needed with sRGB buffer
    // output_color.rgb = pow(output_color.rgb, vec3(1.0 / 2.2));

    
    return vec4(color, in.a);
}

// Just for testing
fn convertNormalizedLog2ToOpenDomain(color: vec3<f32>, minimum_ev: f32, maximum_ev: f32) -> vec3<f32>
{
    var color = color;
    let in_midgrey = 0.18;
    let total_exposure = maximum_ev - minimum_ev;

    color = (color * total_exposure) + minimum_ev;
    color = pow(vec3(2.0), color);
    color = color * in_midgrey;

    return color;
}