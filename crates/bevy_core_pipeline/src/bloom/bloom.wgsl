#import bevy_core_pipeline::fullscreen_vertex_shader

struct BloomUniforms {
    threshold: f32,
    knee: f32,
    scale: f32,
    intensity: f32,
};

@group(0) @binding(0)
var original: texture_2d<f32>;
@group(0) @binding(1)
var original_sampler: sampler;
@group(0) @binding(2)
var<uniform> uniforms: BloomUniforms;
@group(0) @binding(3)
var up: texture_2d<f32>;

fn quadratic_threshold(color: vec4<f32>, threshold: f32, curve: vec3<f32>) -> vec4<f32> {
    let br = max(max(color.r, color.g), color.b);

    var rq: f32 = clamp(br - curve.x, 0.0, curve.y);
    rq = curve.z * rq * rq;

    return color * max(rq, br - threshold) / max(br, 0.0001);
}

// Samples original around the supplied uv using a filter.
//
// o   o   o
//   o   o
// o   o   o
//   o   o
// o   o   o
//
// This is used because it has a number of advantages that
// outweigh the cost of 13 samples that basically boil down
// to it looking better.
//
// These advantages are outlined in a youtube video by the Cherno:
//   https://www.youtube.com/watch?v=tI70-HIc5ro
fn sample_13_tap(uv: vec2<f32>, scale: vec2<f32>) -> vec4<f32> {
    let a = textureSample(original, original_sampler, uv + vec2<f32>(-1.0, -1.0) * scale);
    let b = textureSample(original, original_sampler, uv + vec2<f32>(0.0, -1.0) * scale);
    let c = textureSample(original, original_sampler, uv + vec2<f32>(1.0, -1.0) * scale);
    let d = textureSample(original, original_sampler, uv + vec2<f32>(-0.5, -0.5) * scale);
    let e = textureSample(original, original_sampler, uv + vec2<f32>(0.5, -0.5) * scale);
    let f = textureSample(original, original_sampler, uv + vec2<f32>(-1.0, 0.0) * scale);
    let g = textureSample(original, original_sampler, uv + vec2<f32>(0.0, 0.0) * scale);
    let h = textureSample(original, original_sampler, uv + vec2<f32>(1.0, 0.0) * scale);
    let i = textureSample(original, original_sampler, uv + vec2<f32>(-0.5, 0.5) * scale);
    let j = textureSample(original, original_sampler, uv + vec2<f32>(0.5, 0.5) * scale);
    let k = textureSample(original, original_sampler, uv + vec2<f32>(-1.0, 1.0) * scale);
    let l = textureSample(original, original_sampler, uv + vec2<f32>(0.0, 1.0) * scale);
    let m = textureSample(original, original_sampler, uv + vec2<f32>(1.0, 1.0) * scale);

    let div = (1.0 / 4.0) * vec2<f32>(0.5, 0.125);

    var o: vec4<f32> = (d + e + i + j) * div.x;
    o = o + (a + b + g + f) * div.y;
    o = o + (b + c + h + g) * div.y;
    o = o + (f + g + l + k) * div.y;
    o = o + (g + h + m + l) * div.y;

    return o;
}

// Samples original using a 3x3 tent filter.
//
// NOTE: Use a 2x2 filter for better perf, but 3x3 looks better.
fn sample_original_3x3_tent(uv: vec2<f32>, scale: vec2<f32>) -> vec4<f32> {
    let d = vec4<f32>(1.0, 1.0, -1.0, 0.0);

    var s: vec4<f32> = textureSample(original, original_sampler, uv - d.xy * scale);
    s = s + textureSample(original, original_sampler, uv - d.wy * scale) * 2.0;
    s = s + textureSample(original, original_sampler, uv - d.zy * scale);

    s = s + textureSample(original, original_sampler, uv + d.zw * scale) * 2.0;
    s = s + textureSample(original, original_sampler, uv) * 4.0;
    s = s + textureSample(original, original_sampler, uv + d.xw * scale) * 2.0;

    s = s + textureSample(original, original_sampler, uv + d.zy * scale);
    s = s + textureSample(original, original_sampler, uv + d.wy * scale) * 2.0;
    s = s + textureSample(original, original_sampler, uv + d.xy * scale);

    return s / 16.0;
}

@fragment
fn downsample_prefilter(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
    let texel_size = 1.0 / vec2<f32>(textureDimensions(original));

    let scale = texel_size;

    let curve = vec3<f32>(
        uniforms.threshold - uniforms.knee,
        uniforms.knee * 2.0,
        0.25 / uniforms.knee,
    );

    var o: vec4<f32> = sample_13_tap(uv, scale);

    o = quadratic_threshold(o, uniforms.threshold, curve);
    o = max(o, vec4<f32>(0.00001));

    return o;
}

@fragment
fn downsample(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
    let texel_size = 1.0 / vec2<f32>(textureDimensions(original));

    let scale = texel_size;

    return sample_13_tap(uv, scale);
}

@fragment
fn upsample(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
    let texel_size = 1.0 / vec2<f32>(textureDimensions(original));

    let upsample = sample_original_3x3_tent(uv, texel_size * uniforms.scale);
    var color: vec4<f32> = textureSample(up, original_sampler, uv);
    color = vec4<f32>(color.rgb + upsample.rgb, upsample.a);

    return color;
}

@fragment
fn upsample_final(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
    let texel_size = 1.0 / vec2<f32>(textureDimensions(original));

    let upsample = sample_original_3x3_tent(uv, texel_size * uniforms.scale);

    return vec4<f32>(upsample.rgb * uniforms.intensity, upsample.a);
}
