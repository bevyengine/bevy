// Reference: https://learnopengl.com/Guest-Articles/2022/Phys.-Based-Bloom

#import bevy_core_pipeline::fullscreen_vertex_shader

struct BloomSettings {
    intensity: f32,
    threshold: f32,
    knee: f32,
};

@group(0) @binding(0)
var input_texture: texture_2d<f32>;
@group(0) @binding(1)
var s: sampler;
@group(0) @binding(2)
var<uniform> settings: BloomSettings;
@group(0) @binding(3)
var main_pass_texture: texture_2d<f32>;

fn quadratic_threshold(color: vec3<f32>, threshold: f32, curve: vec3<f32>) -> vec3<f32> {
    let br = max(max(color.r, color.g), color.b);

    var rq: f32 = clamp(br - curve.x, 0.0, curve.y);
    rq = curve.z * rq * rq;

    return color * max(rq, br - threshold) / max(br, 0.0001);
}

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

    var sample = e * 0.125;
    sample += (a + c + g + i) * 0.03125;
    sample += (b + d + f + h) * 0.0625;
    sample += (j + k + l + m) * 0.125;
    return sample;
}

fn sample_input_3x3_tent(uv: vec2<f32>) -> vec3<f32> {
    let d = vec4<f32>(1.0, 1.0, -1.0, 0.0);

    var sample: vec3<f32> = textureSample(input_texture, s, uv - d.xy).rgb;
    sample += textureSample(input_texture, s, uv - d.wy).rgb * 2.0;
    sample += textureSample(input_texture, s, uv - d.zy).rgb;

    sample += textureSample(input_texture, s, uv + d.zw).rgb * 2.0;
    sample += textureSample(input_texture, s, uv).rgb * 4.0;
    sample += textureSample(input_texture, s, uv + d.xw).rgb * 2.0;

    sample += textureSample(input_texture, s, uv + d.zy).rgb;
    sample += textureSample(input_texture, s, uv + d.wy).rgb * 2.0;
    sample += textureSample(input_texture, s, uv + d.xy).rgb;

    return sample / 16.0;
}

@fragment
fn downsample_first(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
    // let texel_size = 1.0 / vec2<f32>(textureDimensions(original));

    // let scale = texel_size;

    // let curve = vec3<f32>(
    //     settings.threshold - settings.knee,
    //     settings.knee * 2.0,
    //     0.25 / settings.knee,
    // );

    // var o: vec3<f32> = sample_13_tap(uv, scale);

    // var q = quadratic_threshold(o.rgb, settings.threshold, curve);
    // q = max(q, vec3<f32>(0.00001));

    // return vec4<f32>(q, 0.0);

    return vec4<f32>(sample_input_13_tap(uv), 1.0);
}

@fragment
fn downsample(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
    return vec4<f32>(sample_input_13_tap(uv), 1.0);
}

@fragment
fn upsample(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
    // let texel_size = 1.0 / vec2<f32>(textureDimensions(original));

    // let upsample = sample_original_3x3_tent(uv, texel_size);
    // var color: vec4<f32> = textureSample(up, original_sampler, uv);
    // color = vec4<f32>(color.rgb + upsample.rgb, 0.0);

    return vec4<f32>(sample_input_3x3_tent(uv), 1.0);
}

@fragment
fn upsample_final(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
    let main_pass_sample = textureSample(main_pass_texture, s, uv);
    let bloom_sample = sample_input_13_tap(uv);

    let sample = mix(main_pass_sample.rgb, bloom_sample, settings.intensity);

    return vec4<f32>(sample, main_pass_sample.a);
}
