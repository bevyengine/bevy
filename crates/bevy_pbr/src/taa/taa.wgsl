#import bevy_core_pipeline::fullscreen_vertex_shader
#import bevy_core_pipeline::tonemapping

@group(0) @binding(0) var view_target: texture_2d<f32>;
@group(0) @binding(1) var taa_accumulation: texture_2d<f32>;
@group(0) @binding(2) var velocity: texture_2d<f32>;
@group(0) @binding(3) var nearest_sampler: sampler;
@group(0) @binding(4) var linear_sampler: sampler;

// The following 3 functions are from Playdead
// https://github.com/playdeadgames/temporal/blob/master/Assets/Shaders/TemporalReprojection.shader
fn RGB_to_YCoCg(rgb: vec3<f32>) -> vec3<f32> {
    let y = (rgb.r / 4.0) + (rgb.g / 2.0) + (rgb.b / 4.0);
    let co = (rgb.r / 2.0) - (rgb.b / 2.0);
    let cg = (-rgb.r / 4.0) + (rgb.g / 2.0) - (rgb.b / 4.0);
    return vec3(y, co, cg);
}

fn YCoCg_to_RGB(ycocg: vec3<f32>) -> vec3<f32> {
    let r = ycocg.x + ycocg.y - ycocg.z;
    let g = ycocg.x + ycocg.z;
    let b = ycocg.x - ycocg.y - ycocg.z;
    return saturate(vec3(r, g, b));
}

fn clip_towards_aabb_center(previous_color: vec3<f32>, current_color: vec3<f32>, aabb_min: vec3<f32>, aabb_max: vec3<f32>) -> vec3<f32> {
    let p_clip = 0.5 * (aabb_max + aabb_min);
    let e_clip = 0.5 * (aabb_max - aabb_min) + 0.00000001;
    let v_clip = previous_color - p_clip;
    let v_unit = v_clip / e_clip;
    let a_unit = abs(v_unit);
    let ma_unit = max(a_unit.x, max(a_unit.y, a_unit.z));
    if ma_unit > 1.0 {
        return p_clip + v_clip / ma_unit;
    } else {
        return previous_color;
    }
}

fn sample_view_target(uv: vec2<f32>) -> vec3<f32> {
    let c = textureSample(view_target, nearest_sampler, uv).rgb;
#ifdef TONEMAP
    let c = reinhard_luminance(c);
#endif
    return RGB_to_YCoCg(c);
}

@fragment
fn taa(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
    let texture_size = vec2<f32>(textureDimensions(view_target));
    let texel_size = 1.0 / texture_size;

    // Fetch the current sample
    let original_color = textureSample(view_target, nearest_sampler, uv);
    let current_color = original_color.rgb;
#ifdef TONEMAP
    let current_color = reinhard_luminance(current_color);
#endif

    // Reproject to find the equivalent sample from the past, using 5-tap Catmull-Rom filtering
    // from https://gist.github.com/TheRealMJP/c83b8c0f46b63f3a88a5986f4fa982b1
    // and https://www.activision.com/cdn/research/Dynamic_Temporal_Antialiasing_and_Upsampling_in_Call_of_Duty_v4.pdf#page=68
    let current_velocity = textureSample(velocity, nearest_sampler, uv).rg;
    let sample_position = (uv + current_velocity) * texture_size;
    let texel_position_1 = floor(sample_position - 0.5) + 0.5;
    let f = sample_position - texel_position_1;
    let w0 = f * (-0.5 + f * (1.0 - 0.5 * f));
    let w1 = 1.0 + f * f * (-2.5 + 1.5 * f);
    let w2 = f * (0.5 + f * (2.0 - 1.5 * f));
    let w3 = f * f * (-0.5 + 0.5 * f);
    let w12 = w1 + w2;
    let offset12 = w2 / (w1 + w2);
    let texel_position_0 = (texel_position_1 - 1.0) * texel_size;
    let texel_position_3 = (texel_position_1 + 2.0) * texel_size;
    let texel_position_12 = (texel_position_1 + offset12) * texel_size;
    var previous_color = vec3(0.0);
    previous_color += textureSample(taa_accumulation, linear_sampler, vec2(texel_position_12.x, texel_position_0.y)).rgb * w12.x * w0.y;
    previous_color += textureSample(taa_accumulation, linear_sampler, vec2(texel_position_0.x, texel_position_12.y)).rgb * w0.x * w12.y;
    previous_color += textureSample(taa_accumulation, linear_sampler, vec2(texel_position_12.x, texel_position_12.y)).rgb * w12.x * w12.y;
    previous_color += textureSample(taa_accumulation, linear_sampler, vec2(texel_position_3.x, texel_position_12.y)).rgb * w3.x * w12.y;
    previous_color += textureSample(taa_accumulation, linear_sampler, vec2(texel_position_12.x, texel_position_3.y)).rgb * w12.x * w3.y;

    // Constrain past sample with 3x3 YCoCg variance clipping to handle disocclusion
    let s_tl = sample_view_target(uv + vec2(-texel_size.x, texel_size.y));
    let s_tm = sample_view_target(uv + vec2(0.0, texel_size.y));
    let s_tr = sample_view_target(uv + texel_size);
    let s_ml = sample_view_target(uv - vec2(texel_size.x, 0.0));
    let s_mm = RGB_to_YCoCg(current_color);
    let s_mr = sample_view_target(uv + vec2(texel_size.x, 0.0));
    let s_bl = sample_view_target(uv - texel_size);
    let s_bm = sample_view_target(uv - vec2(0.0, texel_size.y));
    let s_br = sample_view_target(uv + vec2(texel_size.x, -texel_size.y));
    let moment_1 = s_tl + s_tm + s_tr + s_ml + s_mm + s_mr + s_bl + s_bm + s_br;
    let moment_2 = (s_tl * s_tl) + (s_tm * s_tm) + (s_tr * s_tr) + (s_ml * s_ml) + (s_mm * s_mm) + (s_mr * s_mr) + (s_bl * s_bl) + (s_bm * s_bm) + (s_br * s_br);
    let mean = moment_1 / 9.0;
    let variance = sqrt((moment_2 / 9.0) - (mean * mean));
    let previous_color = RGB_to_YCoCg(previous_color);
    let previous_color = clip_towards_aabb_center(previous_color, s_mm, mean - variance, mean + variance);
    let previous_color = YCoCg_to_RGB(previous_color);

    // Blend current and past sample
    let output = (current_color * 0.1) + (previous_color * 0.9);

    return vec4<f32>(output, original_color.a);
}

// ----------------------------------------------------------------------------

@group(0) @binding(0) var taa_output: texture_2d<f32>;
@group(0) @binding(1) var blit_sampler: sampler;

struct BlitOutput {
    @location(0) view_target: vec4<f32>,
    @location(1) taa_accumulation: vec4<f32>,
}

@fragment
fn blit(@location(0) uv: vec2<f32>) -> BlitOutput {
    var out: BlitOutput;

    out.taa_accumulation = textureSample(taa_output, blit_sampler, uv);

#ifdef TONEMAP
    out.view_target = vec4(inverse_reinhard_luminance(out.taa_accumulation.rgb), out.taa_accumulation.a);
#else
    out.view_target = out.taa_accumulation;
#endif

    return out;
}
