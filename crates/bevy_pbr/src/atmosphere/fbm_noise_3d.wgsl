// 2D FBM (Fractional Brownian Motion) Noise Generator
// Generates a 2D noise texture for cloud coverage

#import bevy_pbr::atmosphere::{
    types::AtmosphereSettings,
    bindings::settings,
}

@group(0) @binding(13) var noise_texture_out: texture_storage_2d<r16float, write>;

// Parameters for FBM noise generation
struct FbmParams {
    octaves: u32,
    frequency: f32,
    amplitude: f32,
    lacunarity: f32,
    persistence: f32,
}

@group(0) @binding(14) var<uniform> fbm_params: FbmParams;

@compute
@workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let texture_size = textureDimensions(noise_texture_out);
    
    if (gid.x >= texture_size.x || gid.y >= texture_size.y) {
        return;
    }
    
    // Normalized coordinates [0, 1]
    let uv = (vec2<f32>(gid.xy) + 0.5) / vec2<f32>(texture_size);
    
    // Generate FBM noise
    let noise_value = fbm_2d(uv * fbm_params.frequency);
    
    // Store the noise value in the R channel.
    // (For single-channel storage textures, only `.x` is written/used.)
    textureStore(noise_texture_out, vec2<i32>(gid.xy), vec4(noise_value, 0.0, 0.0, 1.0));
}

// 2D FBM noise function
fn fbm_2d(p: vec2<f32>) -> f32 {
    var value = 0.0;
    var amplitude = fbm_params.amplitude;
    var frequency = 1.0;
    var position = p;
    
    for (var i = 0u; i < fbm_params.octaves; i++) {
        value += amplitude * value_noise_2d(position * frequency);
        frequency *= fbm_params.lacunarity;
        amplitude *= fbm_params.persistence;
    }
    
    // Normalize to [0, 1] range.
    // Note: the underlying noise sum can overshoot slightly; clamp to keep downstream
    // shaping (pow/smoothstep) well-defined.
    return clamp(value * 0.5 + 0.5, 0.0, 1.0);
}

fn value_noise_2d(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);

    let a = hash_2d(i + vec2(0.0, 0.0));
    let b = hash_2d(i + vec2(1.0, 0.0));
    let c = hash_2d(i + vec2(0.0, 1.0));
    let d = hash_2d(i + vec2(1.0, 1.0));

    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y) * 2.0 - 1.0;
}

fn hash_2d(p: vec2<f32>) -> f32 {
    let p3 = fract(vec3(p.x, p.y, p.x) * 0.1031);
    let s = dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * (p3.z + s));
}

