// 3D FBM (Fractional Brownian Motion) Noise Generator
// Generates a 3D noise texture for atmospheric effects

#import bevy_pbr::atmosphere::{
    types::AtmosphereSettings,
    bindings::settings,
}

@group(0) @binding(13) var noise_texture_out: texture_storage_3d<rgba16float, write>;

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
@workgroup_size(4, 4, 4)
fn main(@builtin(global_invocation_id) idx: vec3<u32>) {
    let texture_size = textureDimensions(noise_texture_out);
    
    if (idx.x >= texture_size.x || idx.y >= texture_size.y || idx.z >= texture_size.z) {
        return;
    }
    
    // Normalized coordinates [0, 1]
    let uvw = (vec3<f32>(idx) + 0.5) / vec3<f32>(texture_size);
    
    // Generate FBM noise
    let noise_value = fbm_3d(uvw * fbm_params.frequency);
    
    // Store the noise value in all channels (can be used for different purposes)
    textureStore(noise_texture_out, idx, vec4(noise_value, noise_value, noise_value, 1.0));
}

// 3D FBM noise function
fn fbm_3d(p: vec3<f32>) -> f32 {
    var value = 0.0;
    var amplitude = fbm_params.amplitude;
    var frequency = 1.0;
    var position = p;
    
    for (var i = 0u; i < fbm_params.octaves; i++) {
        value += amplitude * perlin_noise_3d(position * frequency);
        frequency *= fbm_params.lacunarity;
        amplitude *= fbm_params.persistence;
    }
    
    // Normalize to [0, 1] range
    return value * 0.5 + 0.5;
}

// 3D Perlin noise implementation
fn perlin_noise_3d(p: vec3<f32>) -> f32 {
    let pi = floor(p);
    let pf = fract(p);
    
    // Fade curve
    let u = fade(pf);
    
    // Hash coordinates of the 8 cube corners
    let h000 = hash_3d(pi + vec3(0.0, 0.0, 0.0));
    let h100 = hash_3d(pi + vec3(1.0, 0.0, 0.0));
    let h010 = hash_3d(pi + vec3(0.0, 1.0, 0.0));
    let h110 = hash_3d(pi + vec3(1.0, 1.0, 0.0));
    let h001 = hash_3d(pi + vec3(0.0, 0.0, 1.0));
    let h101 = hash_3d(pi + vec3(1.0, 0.0, 1.0));
    let h011 = hash_3d(pi + vec3(0.0, 1.0, 1.0));
    let h111 = hash_3d(pi + vec3(1.0, 1.0, 1.0));
    
    // Gradients
    let g000 = gradient_3d(h000, pf - vec3(0.0, 0.0, 0.0));
    let g100 = gradient_3d(h100, pf - vec3(1.0, 0.0, 0.0));
    let g010 = gradient_3d(h010, pf - vec3(0.0, 1.0, 0.0));
    let g110 = gradient_3d(h110, pf - vec3(1.0, 1.0, 0.0));
    let g001 = gradient_3d(h001, pf - vec3(0.0, 0.0, 1.0));
    let g101 = gradient_3d(h101, pf - vec3(1.0, 0.0, 1.0));
    let g011 = gradient_3d(h011, pf - vec3(0.0, 1.0, 1.0));
    let g111 = gradient_3d(h111, pf - vec3(1.0, 1.0, 1.0));
    
    // Trilinear interpolation
    let x00 = mix(g000, g100, u.x);
    let x10 = mix(g010, g110, u.x);
    let x01 = mix(g001, g101, u.x);
    let x11 = mix(g011, g111, u.x);
    
    let y0 = mix(x00, x10, u.y);
    let y1 = mix(x01, x11, u.y);
    
    return mix(y0, y1, u.z);
}

// Fade function for smooth interpolation
fn fade(t: vec3<f32>) -> vec3<f32> {
    return t * t * t * (t * (t * 6.0 - 15.0) + 10.0);
}

// Hash function for generating pseudo-random values
fn hash_3d(p: vec3<f32>) -> f32 {
    let p3 = fract(p * 0.1031);
    var p_sum = p3.x + p3.y + p3.z;
    p_sum = p3.x * p3.y * p3.z + p_sum;
    return fract(sin(p_sum) * 43758.5453123);
}

// Gradient function for Perlin noise
fn gradient_3d(hash: f32, p: vec3<f32>) -> f32 {
    // Convert hash to gradient direction
    let h = u32(hash * 16.0);
    let u = select(p.y, p.x, (h & 8u) == 0u);
    
    var v: f32;
    if ((h & 8u) == 0u) {
        if ((h & 4u) == 0u) {
            v = p.y;
        } else {
            v = p.z;
        }
    } else {
        if ((h & 4u) == 0u) {
            v = p.x;
        } else {
            v = p.z;
        }
    }
    
    let u_sign = select(-u, u, (h & 1u) == 0u);
    let v_sign = select(-v, v, (h & 2u) == 0u);
    
    return u_sign + v_sign;
}

