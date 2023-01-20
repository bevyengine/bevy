// References:
// https://www.elopezr.com/temporal-aa-and-the-quest-for-the-holy-trail
// http://behindthepixels.io/assets/files/TemporalAA.pdf
// http://leiy.cc/publications/TAA/TAA_EG2020_Talk.pdf

#import bevy_core_pipeline::fullscreen_vertex_shader
#import bevy_core_pipeline::tonemapping

@group(0) @binding(0) var view_target: texture_2d<f32>;
@group(0) @binding(1) var history: texture_2d<f32>;
@group(0) @binding(2) var velocity: texture_2d<f32>;
@group(0) @binding(3) var depth: texture_depth_2d;
@group(0) @binding(4) var nearest_sampler: sampler;
@group(0) @binding(5) var linear_sampler: sampler;

struct Output {
    @location(0) view_target: vec4<f32>,
    @location(1) history: vec4<f32>,
};

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
    return select(previous_color, p_clip + v_clip / ma_unit, ma_unit > 1.0);
}

fn sample_view_target(uv: vec2<f32>) -> vec3<f32> {
    var sample = textureSample(view_target, nearest_sampler, uv).rgb;
#ifdef TONEMAP
    let sample = reinhard_luminance(sample);
#endif
    return RGB_to_YCoCg(sample);
}

@fragment
fn taa(@location(0) uv: vec2<f32>) -> Output {
    let texture_size = vec2<f32>(textureDimensions(view_target));
    let texel_size = 1.0 / texture_size;

    // Fetch the current sample
    let original_color = textureSample(view_target, nearest_sampler, uv);
    let current_color = original_color.rgb;
#ifdef TONEMAP
    let current_color = reinhard_luminance(current_color);
#endif

    // Pick the closest velocity from 5 samples (reduces aliasing on the edges of moving entities)
    // https://advances.realtimerendering.com/s2014/index.html#_HIGH-QUALITY_TEMPORAL_SUPERSAMPLING, slide 27
    let offset = texel_size * 2.0;
    let v_tl = textureSample(velocity, nearest_sampler, uv + vec2(-offset.x, offset.y)).rg;
    let v_tr = textureSample(velocity, nearest_sampler, uv + vec2(offset.x, offset.y)).rg;
    var closest_velocity = textureSample(velocity, nearest_sampler, uv).rg;
    let v_bl = textureSample(velocity, nearest_sampler, uv + vec2(-offset.x, -offset.y)).rg;
    let v_br = textureSample(velocity, nearest_sampler, uv + vec2(offset.x, -offset.y)).rg;
    let d_tl = textureSample(depth, nearest_sampler, uv + vec2(-offset.x, offset.y));
    let d_tr = textureSample(depth, nearest_sampler, uv + vec2(offset.x, offset.y));
    let current_depth = textureSample(depth, nearest_sampler, uv);
    var closest_depth = current_depth;
    let d_bl = textureSample(depth, nearest_sampler, uv + vec2(-offset.x, -offset.y));
    let d_br = textureSample(depth, nearest_sampler, uv + vec2(offset.x, -offset.y));
    if d_tl > closest_depth {
        closest_velocity = v_tl;
        closest_depth = d_tl;
    }
    if d_tr > closest_depth {
        closest_velocity = v_tr;
        closest_depth = d_tr;
    }
    if d_bl > closest_depth {
        closest_velocity = v_bl;
        closest_depth = d_bl;
    }
    if d_br > closest_depth {
        closest_velocity = v_br;
    }
    let previous_uv = uv - closest_velocity;

    // Reproject to find the equivalent sample from the past
    // Uses 5-sample Catmull-Rom filtering (reduces blurriness)
    // https://gist.github.com/TheRealMJP/c83b8c0f46b63f3a88a5986f4fa982b1
    // https://vec3.ca/bicubic-filtering-in-fewer-taps
    // https://developer.nvidia.com/gpugems/gpugems2/part-iii-high-quality-rendering/chapter-20-fast-third-order-texture-filtering
    // https://www.activision.com/cdn/research/Dynamic_Temporal_Antialiasing_and_Upsampling_in_Call_of_Duty_v4.pdf#page=68
    let sample_position = previous_uv * texture_size;
    let texel_center = floor(sample_position - 0.5) + 0.5;
    let f = sample_position - texel_center;
    let w0 = f * (-0.5 + f * (1.0 - 0.5 * f));
    let w1 = 1.0 + f * f * (-2.5 + 1.5 * f);
    let w2 = f * (0.5 + f * (2.0 - 1.5 * f));
    let w3 = f * f * (-0.5 + 0.5 * f);
    let w12 = w1 + w2;
    let texel_position_0 = (texel_center - 1.0) * texel_size;
    let texel_position_3 = (texel_center + 2.0) * texel_size;
    let texel_position_12 = (texel_center + (w2 / w12)) * texel_size;
    var previous_color = vec3(0.0);
    previous_color += textureSample(history, linear_sampler, vec2(texel_position_12.x, texel_position_0.y)).rgb * w12.x * w0.y;
    previous_color += textureSample(history, linear_sampler, vec2(texel_position_0.x, texel_position_12.y)).rgb * w0.x * w12.y;
    previous_color += textureSample(history, linear_sampler, vec2(texel_position_12.x, texel_position_12.y)).rgb * w12.x * w12.y;
    previous_color += textureSample(history, linear_sampler, vec2(texel_position_3.x, texel_position_12.y)).rgb * w3.x * w12.y;
    previous_color += textureSample(history, linear_sampler, vec2(texel_position_12.x, texel_position_3.y)).rgb * w12.x * w3.y;

    // Constrain past sample with 3x3 YCoCg variance clipping (reduces ghosting)
    // YCoCg: https://advances.realtimerendering.com/s2014/index.html#_HIGH-QUALITY_TEMPORAL_SUPERSAMPLING, slide 33
    // Variance clipping: https://developer.download.nvidia.com/gameworks/events/GDC2016/msalvi_temporal_supersampling.pdf
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
    previous_color = RGB_to_YCoCg(previous_color);
    previous_color = clip_towards_aabb_center(previous_color, s_mm, mean - variance, mean + variance);
    previous_color = YCoCg_to_RGB(previous_color);

    // Blend current and past sample
    let output = mix(previous_color, current_color, 0.1);

    // Write output to history and view target
    var out: Output;
    out.history = vec4(output, original_color.a);
#ifdef TONEMAP
    out.view_target = vec4(inverse_reinhard_luminance(out.history.rgb), out.history.a);
#else
    out.view_target = out.history;
#endif
    return out;
}
