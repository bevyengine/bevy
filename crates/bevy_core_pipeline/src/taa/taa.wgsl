// References:
// https://www.elopezr.com/temporal-aa-and-the-quest-for-the-holy-trail
// https://alextardif.com/TAA.html
// http://behindthepixels.io/assets/files/TemporalAA.pdf
// http://leiy.cc/publications/TAA/TAA_EG2020_Talk.pdf
// https://advances.realtimerendering.com/s2014/index.html#_HIGH-QUALITY_TEMPORAL_SUPERSAMPLING

@group(0) @binding(0) var view_target: texture_2d<f32>;
@group(0) @binding(1) var history: texture_2d<f32>;
@group(0) @binding(2) var motion_vectors: texture_2d<f32>;
@group(0) @binding(3) var depth: texture_depth_2d;
@group(0) @binding(4) var nearest_sampler: sampler;
@group(0) @binding(5) var linear_sampler: sampler;
@group(0) @binding(6) var<uniform> user_reset: f32;

struct Output {
    @location(0) view_target: vec4<f32>,
    @location(1) history: vec4<f32>,
};

// TAA is ideally applied after tonemapping (if not tonemapping in the main pass), but before post processing
// Post processing wants to go before tonemapping, which conflicts
// Solution: Put TAA before tonemapping, tonemap TAA input, apply TAA, invert-tonemap TAA output
// https://advances.realtimerendering.com/s2014/index.html#_HIGH-QUALITY_TEMPORAL_SUPERSAMPLING, slide 20
// https://gpuopen.com/learn/optimized-reversible-tonemapper-for-resolve
fn rcp(x: f32) -> f32 { return 1.0 / x; }
fn max3(x: vec3<f32>) -> f32 { return max(x.r, max(x.g, x.b)); }
fn tonemap(color: vec3<f32>) -> vec3<f32> { return color * rcp(max3(color) + 1.0); }
fn reverse_tonemap(color: vec3<f32>) -> vec3<f32> { return color * rcp(1.0 - max3(color)); }

// The following 3 functions are from Playdead (MIT-licensed)
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

fn clip_towards_aabb_center(history_color: vec3<f32>, current_color: vec3<f32>, aabb_min: vec3<f32>, aabb_max: vec3<f32>) -> vec3<f32> {
    let p_clip = 0.5 * (aabb_max + aabb_min);
    let e_clip = 0.5 * (aabb_max - aabb_min) + 0.00000001;
    let v_clip = history_color - p_clip;
    let v_unit = v_clip / e_clip;
    let a_unit = abs(v_unit);
    let ma_unit = max3(a_unit);
    if ma_unit > 1.0 {
        return p_clip + (v_clip / ma_unit);
    } else {
        return history_color;
    }
}

fn sample_history(u: f32, v: f32) -> vec3<f32> {
    return textureSampleLevel(history, linear_sampler, vec2(u, v), 0.0).rgb;
}

fn sample_view_target(uv: vec2<f32>) -> vec3<f32> {
    var sample = textureSampleLevel(view_target, nearest_sampler, uv, 0.0).rgb;
#ifdef TONEMAP
    sample = tonemap(sample);
#endif
    return RGB_to_YCoCg(sample);
}

@fragment
fn taa(@location(0) uv: vec2<f32>) -> Output {
    let texture_size = vec2<f32>(textureDimensions(view_target));
    let texel_size = 1.0 / texture_size;

    // Loop over 3x3 neighborhood of the pre-TAA rendered texture
    // https://alextardif.com/TAA.html
    var current_color = vec3(0.0);
    var moment_1 = vec3(0.0);
    var moment_2 = vec3(0.0);
    var closest_depth = 0.0;
    var closest_uv = uv;
    var weights = array(0.05556, 0.88889, 0.05556);
    for (var x = -1.0; x <= 1.0; x += 1.0) {
        for (var y = -1.0; y <= 1.0; y += 1.0) {
            let sample_uv = uv + (vec2(x, y) * texel_size);
            let sample = sample_view_target(sample_uv);

            // Apply Mitchell-Netravali kernel over the jittered 3x3 neighborhood to reduce softness
            let weight = weights[u32(x + 1.0)] * weights[u32(y + 1.0)];
            current_color += sample * weight;

            // Calculate first and second color moments for use with variance clipping
            moment_1 += sample;
            moment_2 += sample * sample;

            // Find closest pixel to take motion vectors from (reduces aliasing on the edges of moving entities)
            let sample_depth = textureSampleLevel(depth, nearest_sampler, sample_uv, 0.0);
            if sample_depth > closest_depth {
                closest_depth = sample_depth;
                closest_uv = sample_uv;
            }
        }
    }

    // Reproject to find the equivalent sample from the past
    // Uses 5-sample Catmull-Rom filtering (reduces blurriness)
    // Catmull-Rom filtering: https://gist.github.com/TheRealMJP/c83b8c0f46b63f3a88a5986f4fa982b1
    // Ignoring corners: https://www.activision.com/cdn/research/Dynamic_Temporal_Antialiasing_and_Upsampling_in_Call_of_Duty_v4.pdf#page=68
    // Technically we should renormalize the weights since we're skipping the corners, but it's basically the same result
    let history_uv = uv - textureSampleLevel(motion_vectors, nearest_sampler, closest_uv, 0.0).rg;
    let sample_position = history_uv * texture_size;
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
    var history_color = sample_history(texel_position_12.x, texel_position_0.y) * w12.x * w0.y;
    history_color += sample_history(texel_position_0.x, texel_position_12.y) * w0.x * w12.y;
    history_color += sample_history(texel_position_12.x, texel_position_12.y) * w12.x * w12.y;
    history_color += sample_history(texel_position_3.x, texel_position_12.y) * w3.x * w12.y;
    history_color += sample_history(texel_position_12.x, texel_position_3.y) * w12.x * w3.y;

    // Constrain past sample with 3x3 YCoCg variance clipping (reduces ghosting)
    // YCoCg: https://advances.realtimerendering.com/s2014/index.html#_HIGH-QUALITY_TEMPORAL_SUPERSAMPLING, slide 33
    // Variance clipping: https://developer.download.nvidia.com/gameworks/events/GDC2016/msalvi_temporal_supersampling.pdf
    let mean = moment_1 / 9.0;
    let variance = (moment_2 / 9.0) - (mean * mean);
    let std_deviation = sqrt(max(variance, vec3(0.0)));
    history_color = clip_towards_aabb_center(history_color, current_color, mean - std_deviation, mean + std_deviation);

    // Use more of the history if it's been visible for a few frames (reduces noise)
    var accumulated_samples = textureSampleLevel(history, nearest_sampler, history_uv, 0.0).a;
    // If the history_uv is pointing off-screen, reset accumulated sample count
    accumulated_samples *= f32(all(saturate(history_uv) == history_uv));
    accumulated_samples *= user_reset;
    accumulated_samples = max(accumulated_samples + 1.0, 8.0);

    // Blend current and past sample
    current_color = mix(history_color, current_color, 1.0 / accumulated_samples);

    // Write output to history and view target
    var out: Output;
    out.history = vec4(current_color, accumulated_samples);
    current_color = YCoCg_to_RGB(current_color);
#ifdef TONEMAP
    current_color = reverse_tonemap(current_color);
#endif
    out.view_target = vec4(current_color, 1.0);
    return out;
}
