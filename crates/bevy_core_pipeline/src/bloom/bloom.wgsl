// Bloom works by creating an intermediate texture with a bunch of mip levels, each half the size of the previous.
// You then downsample each mip (starting with the original texture) to the lower resolution mip under it, going in order.
// You then upsample each mip (starting from the smallest mip) and blend with the higher resolution mip above it (ending on the original texture).
//
// References:
// * http://www.iryoku.com/next-generation-post-processing-in-call-of-duty-advanced-warfare
// * https://learnopengl.com/Guest-Articles/2022/Phys.-Based-Bloom

#import bevy_core_pipeline::fullscreen_vertex_shader
#import bevy_core_pipeline::tonemapping

struct BloomUniforms {
    threshold_precomputations: vec4<f32>,
    viewport: vec4<f32>,
};

@group(0) @binding(0)
var input_texture: texture_2d<f32>;
@group(0) @binding(1)
var s: sampler;

#ifdef FIRST_DOWNSAMPLE
@group(0) @binding(2)
var<uniform> uniforms: BloomUniforms;

// https://catlikecoding.com/unity/tutorials/advanced-rendering/bloom/#3.4
fn soft_threshold(color: vec3<f32>) -> vec3<f32> {
    let brightness = max(color.r, max(color.g, color.b));
    var softness = brightness - uniforms.threshold_precomputations.y;
    softness = clamp(softness, 0.0, uniforms.threshold_precomputations.z);
    softness = softness * softness * uniforms.threshold_precomputations.w;
    var contribution = max(brightness - uniforms.threshold_precomputations.x, softness);
    contribution /= max(brightness, 0.00001); // Prevent division by 0
    return color * contribution;
}
#endif

// http://graphicrants.blogspot.com/2013/12/tone-mapping.html
fn karis_average(color: vec3<f32>) -> f32 {
    let luma = tonemapping_luminance(rgb_to_srgb(color)) / 4.0;
    return 1.0 / (1.0 + luma);
}

// "Next Generation Post Processing in Call of Duty: Advanced Warfare" slide 100
fn sample_input_13_tap(uv: vec2<f32>) -> vec3<f32> {
    let texel_size = 1.0 / vec2<f32>(textureDimensions(input_texture));
    let x = texel_size.x;
    let y = texel_size.y;

    let a = textureSample(input_texture, s, vec2<f32>(uv.x - 2.0 * x, uv.y + 2.0 * y)).rgb;
    let b = textureSample(input_texture, s, vec2<f32>(uv.x, uv.y + 2.0 * y)).rgb;
    let c = textureSample(input_texture, s, vec2<f32>(uv.x + 2.0 * x, uv.y + 2.0 * y)).rgb;

    let d = textureSample(input_texture, s, vec2<f32>(uv.x - 2.0 * x, uv.y)).rgb;
    let e = textureSample(input_texture, s, vec2<f32>(uv.x, uv.y)).rgb;
    let f = textureSample(input_texture, s, vec2<f32>(uv.x + 2.0 * x, uv.y)).rgb;

    let g = textureSample(input_texture, s, vec2<f32>(uv.x - 2.0 * x, uv.y - 2.0 * y)).rgb;
    let h = textureSample(input_texture, s, vec2<f32>(uv.x, uv.y - 2.0 * y)).rgb;
    let i = textureSample(input_texture, s, vec2<f32>(uv.x + 2.0 * x, uv.y - 2.0 * y)).rgb;

    let j = textureSample(input_texture, s, vec2<f32>(uv.x - x, uv.y + y)).rgb;
    let k = textureSample(input_texture, s, vec2<f32>(uv.x + x, uv.y + y)).rgb;
    let l = textureSample(input_texture, s, vec2<f32>(uv.x - x, uv.y - y)).rgb;
    let m = textureSample(input_texture, s, vec2<f32>(uv.x + x, uv.y - y)).rgb;

#ifdef FIRST_DOWNSAMPLE
    // "Next Generation Post Processing in Call of Duty: Advanced Warfare" slide 112
    var group0 = (a + b + d + e) * (0.125f / 4.0f);
    var group1 = (b + c + e + f) * (0.125f / 4.0f);
    var group2 = (d + e + g + h) * (0.125f / 4.0f);
    var group3 = (e + f + h + i) * (0.125f / 4.0f);
    var group4 = (j + k + l + m) * (0.5f / 4.0f);
    group0 *= karis_average(group0);
    group1 *= karis_average(group1);
    group2 *= karis_average(group2);
    group3 *= karis_average(group3);
    group4 *= karis_average(group4);
    return group0 + group1 + group2 + group3 + group4;
#else
    var sample = e * 0.125;
    sample += (a + c + g + i) * 0.03125;
    sample += (b + d + f + h) * 0.0625;
    sample += (j + k + l + m) * 0.125;
    return sample;
#endif
}

// "Next Generation Post Processing in Call of Duty: Advanced Warfare" slide 109
fn sample_input_3x3_tent(uv: vec2<f32>) -> vec3<f32> {
    let x = 0.004;
    let y = 0.004;

    let a = textureSample(input_texture, s, vec2<f32>(uv.x - x, uv.y + y)).rgb;
    let b = textureSample(input_texture, s, vec2<f32>(uv.x, uv.y + y)).rgb;
    let c = textureSample(input_texture, s, vec2<f32>(uv.x + x, uv.y + y)).rgb;

    let d = textureSample(input_texture, s, vec2<f32>(uv.x - x, uv.y)).rgb;
    let e = textureSample(input_texture, s, vec2<f32>(uv.x, uv.y)).rgb;
    let f = textureSample(input_texture, s, vec2<f32>(uv.x + x, uv.y)).rgb;

    let g = textureSample(input_texture, s, vec2<f32>(uv.x - x, uv.y - y)).rgb;
    let h = textureSample(input_texture, s, vec2<f32>(uv.x, uv.y - y)).rgb;
    let i = textureSample(input_texture, s, vec2<f32>(uv.x + x, uv.y - y)).rgb;

    var sample = e * 4.0;
    sample += (b + d + f + h) * 2.0;
    sample += (a + c + g + i);
    sample /= 16.0;

    return sample;
}

#ifdef FIRST_DOWNSAMPLE
@fragment
fn downsample_first(@location(0) output_uv: vec2<f32>) -> @location(0) vec4<f32> {
    let sample_uv = uniforms.viewport.xy + output_uv * uniforms.viewport.zw;
    var sample = sample_input_13_tap(sample_uv);
    sample = clamp(sample, vec3<f32>(0.0), vec3<f32>(3.40282347E+38)); // Prevent NaNs

#ifdef USE_THRESHOLD
    sample = soft_threshold(sample);
#endif

    return vec4<f32>(sample, 1.0);
}
#endif

@fragment
fn downsample(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
    return vec4<f32>(sample_input_13_tap(uv), 1.0);
}

@fragment
fn upsample(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
    return vec4<f32>(sample_input_3x3_tent(uv), 1.0);
}
