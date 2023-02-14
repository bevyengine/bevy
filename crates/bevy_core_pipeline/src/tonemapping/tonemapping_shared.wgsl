#define_import_path bevy_core_pipeline::tonemapping

// -------------------------------------
// just some experiment
// -------------------------------------

fn rgb2hsv_v(c: vec3<f32>) -> vec3<f32> {
    let K = vec4(0.0, -1.0 / 3.0, 2.0 / 3.0, -1.0);
    let p = mix(vec4(c.bg, K.wz), vec4(c.gb, K.xy), step(c.b, c.g));
    let q = mix(vec4(p.xyw, c.r), vec4(c.r, p.yzx), step(p.x, c.r));

    let d = q.x - min(q.w, q.y);
    let e = 1.0e-10;
    return vec3(abs(q.z + (q.w - q.y) / (6.0 * d + e)), d / (q.x + e), q.x);
}

fn hsv2rgb_v(c: vec3<f32>) -> vec3<f32> {
    let K = vec4(1.0, 2.0 / 3.0, 1.0 / 3.0, 3.0);
    let p = abs(fract(c.xxx + K.xyz) * 6.0 - K.www);
    return c.z * mix(K.xxx, clamp(p - K.xxx, vec3(0.0), vec3(1.0)), vec3(c.y));
}

fn tonemapping_snidt(color: vec3<f32>) -> vec3<f32> {
    let L = vec3(0.2126, 0.7152, 0.0722 * 4.0);
    var c = max(color, vec3(0.0));
	let m = mat3x3<f32>(
        0.84247906, 0.0784336,  0.07922375, 
        0.04232824, 0.87846864, 0.07916613, 
        0.04237565, 0.0784336,  0.87914297
    );
    c = c * m;

    // Gain, Gamma
    c = powsafe(c * 1.4, 1.03);

    var c2 = rgb2hsv_v(c);
    let og_hsv = c2;

    // Saturation
    c = mix(vec3(saturate(dot(c, L))), c, vec3(1.03));
    
    // reinhard
    c = c / (1.0 + c);

    // Make blues a bit less red
    c.r = c.r - c.b * c.b * og_hsv.y * 0.1;

    // Saturation
    c = mix(vec3(saturate(dot(c, L))), c, vec3(1.15));

    return vec3(c);
}

fn tonemapping_maintain_hue(color: vec3<f32>) -> vec3<f32> {
    var c = color;
    c = rgb2hsv_v(c);
    c.z = c.z / (1.0 + c.z);
    c = hsv2rgb_v(c);
    return c;
}

// -------------------------------------
// ---- tonemapping from kajiya 0.1 ----
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

// Defaults
//const INPUT_EXPOSURE: f32 = 0.0;
//const INPUT_GAMMA: f32  = 1.0;
//const INPUT_SATURATION: f32  = 1.0;
//const INPUT_HIGHLIGHT_GAIN: f32  = 0.0;
//const INPUT_HIGHLIGHT_GAIN_GAMMA: f32  = 1.0;
//const PUNCH_EXPOSURE: f32  = 0.0;
//const PUNCH_SATURATION: f32  = 1.0;
//const PUNCH_GAMMA: f32  = 1.3;

const INPUT_EXPOSURE: f32 = 0.0;
const INPUT_GAMMA: f32  = 1.0; //1.1
const INPUT_SATURATION: f32  = 1.0; //1.1
const INPUT_HIGHLIGHT_GAIN: f32  = 0.0;
const INPUT_HIGHLIGHT_GAIN_GAMMA: f32  = 1.0;
const PUNCH_EXPOSURE: f32  = 0.0;
const PUNCH_SATURATION: f32  = 1.0; //1.1
const PUNCH_GAMMA: f32  = 1.0;



// LUT AgX-default_contrast.lut.png / AgX-default_contrast.lut.exr 

fn getLuminance(image: vec3<f32>) -> f32
// Return approximative perceptive luminance of the image.
{
    return dot(image, vec3(0.2126, 0.7152, 0.0722));
}

fn powsafe(color: vec3<f32>, power: f32) -> vec3<f32>
// pow() but safe for NaNs/negatives
{
    return pow(abs(color), vec3(power)) * sign(color);
}

fn saturation(color: vec3<f32>, saturationAmount: f32) -> vec3<f32>
/*
    Increase color saturation of the given color data.
    :param color: expected sRGB primaries input
    :oaram saturationAmount: expected 0-1 range with 1=neutral, 0=no saturation.
    -- ref[2] [4]
*/
{
    let luma = getLuminance(color);
    return mix(vec3(luma), color, vec3(saturationAmount));
}

fn convertOpenDomainToNormalizedLog2(color: vec3<f32>, minimum_ev: f32, maximum_ev: f32) -> vec3<f32>
/*
    Output log domain encoded data.
    Similar to OCIO lg2 AllocationTransform.
    ref[0]
*/
{
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


fn applyGrading(Image: vec3<f32>) -> vec3<f32>
/*
    Apply creative grading operations (pre-display-transform).
*/
{
    var Image = Image;
    let ImageLuma = powsafe(vec3(getLuminance(Image)), INPUT_HIGHLIGHT_GAIN_GAMMA);
    Image += Image * ImageLuma.xxx * INPUT_HIGHLIGHT_GAIN;

    Image = saturation(Image, INPUT_SATURATION);
    Image = powsafe(Image, INPUT_GAMMA);
    Image *= powsafe(vec3(2.0), INPUT_EXPOSURE);
    return Image;
}

fn applyAgXLog(Image: vec3<f32>) -> vec3<f32>
/*
    Prepare the data for display encoding. Converted to log domain.
*/
{
    var Image = max(vec3(0.0), Image); // clamp negatives
    // why this doesn't work ??
    // Image = mul(agx_compressed_matrix, Image);
	let r = dot(Image, vec3(0.84247906, 0.0784336, 0.07922375));
	let g = dot(Image, vec3(0.04232824, 0.87846864, 0.07916613));
	let b = dot(Image, vec3(0.04237565, 0.0784336, 0.87914297));
	Image = vec3(r, g, b);

    Image = convertOpenDomainToNormalizedLog2(Image, -10.0, 6.5);
    
    Image = clamp(Image, vec3(0.0), vec3(1.0));
    return Image;
}

fn applyAgXLUT3D(Image: vec3<f32>, block_size: f32, dimensions: vec2<f32>, offset: vec2<f32>) -> vec3<f32> {
    return textureSample(dt_lut_texture, dt_lut_sampler, Image * ((block_size - 1.0) / block_size) + 0.5 / block_size).rgb;
}

fn applyLookPunchy(Image: vec3<f32>) -> vec3<f32>
/*
    Applies the post "Punchy" look to display-encoded data.
    Input is expected to be in a display-state.
*/
{
    var Image = powsafe(Image, PUNCH_GAMMA);
    Image = saturation(Image, PUNCH_SATURATION);
    Image *= powsafe(vec3(2.0), PUNCH_EXPOSURE);  // not part of initial cdl
    return Image;

}

fn tonemapping_AgX(Image: vec3<f32>) -> vec3<f32> {
    var Image = Image;
    Image = applyGrading(Image);
    Image = applyAgXLog(Image);
    Image = applyAgXLUT3D(Image, 32.0, vec2<f32>(1024.0, 32.0), vec2(0.0));
    Image = applyLookPunchy(Image);
    return Image;
}

// -------------------------
// -------------------------
// -------------------------

// from https://knarkowicz.wordpress.com/2016/01/06/aces-filmic-tone-mapping-curve
fn tonemapping_aces_knarkowicz(color: vec3<f32>) -> vec3<f32> {
    let a = 2.51;
    let b = 0.03;
    let c = 2.43;
    let d = 0.59;
    let e = 0.14;
    return saturate((color * (a * color + b)) / (color * (c * color + d) + e));
}

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
    var color = max(in, vec4(0.0));
    // tone_mapping
#ifdef TONEMAP_METHOD_NONE
    color = color;
#endif
#ifdef TONEMAP_METHOD_REINHARD
    color = vec4<f32>(tonemapping_reinhard(color.rgb), in.a);
#endif
#ifdef TONEMAP_METHOD_REINHARD_LUMINANCE
    color = vec4<f32>(tonemapping_reinhard_luminance(color.rgb), in.a);
#endif
#ifdef TONEMAP_METHOD_ACES
    // TODO figure out correct value for white here, or factor it out
    color = vec4<f32>(tonemapping_aces_godot_4(color.rgb, 1000.0), in.a);
#endif
#ifdef TONEMAP_METHOD_AGX
    color = vec4<f32>(tonemapping_AgX(color.rgb), in.a);
#endif
#ifdef TONEMAP_METHOD_SBDT
    color = vec4<f32>(tonemapping_sbdt(color.rgb), in.a);
#endif
#ifdef TONEMAP_METHOD_SBDT2
    let block_size = 32.0;
    var c = color.rgb;
    let LUT_EV_RANGE = vec2(-11.0, 8.0);
    c = (log2(c) - LUT_EV_RANGE[0]) / (LUT_EV_RANGE[1] - LUT_EV_RANGE[0]);//convertOpenDomainToNormalizedLog2(c, -11.0, 8.0);
    c = saturate(c);
    c.y = 1.0 - c.y;
    c = applyAgXLUT3D(c, block_size, vec2<f32>(1024.0, 32.0), vec2(1024.0, 0.0));
    c = pow(c, vec3(2.2));
    color = vec4<f32>(c, in.a);
#endif
#ifdef TONEMAP_METHOD_BLENDER_FILMIC
    let block_size = 64.0;
    let selector = 0.0;
    var c = color.rgb; // * 0.82 somewhat matches tonemapping_reinhard_luminance
    c = convertOpenDomainToNormalizedLog2(c, -11.0, 12.0);
    c = saturate(c);
    c = applyAgXLUT3D(c, block_size, vec2<f32>(4096.0, 64.0), vec2(0.0, 32.0 + block_size * selector));
    color = vec4<f32>(c, in.a);
#endif

    // tonemapping_maintain_hue
    // tonemapping_snidt
    // tonemapping_aces_godot_4
    // tonemapping_aces_knarkowicz
    // tonemapping_reinhard
    // tonemapping_reinhard_luminance
    // tonemapping_sbdt

    // Gamma correction.
    // Not needed with sRGB buffer
    // output_color.rgb = pow(output_color.rgb, vec3(1.0 / 2.2));

    
    return color;
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