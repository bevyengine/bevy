// 2D FBM (Fractional Brownian Motion) Noise Generator
// Generates a 2D noise texture for cloud coverage

@group(0) @binding(13) var noise_texture_out: texture_storage_2d<rgba16float, write>;

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
    
    // Normalized coordinates [0, 1)
    let uv = (vec2<f32>(gid.xy) + 0.5) / vec2<f32>(texture_size);

    // Generate packed *tileable* cloud noise.
    // We work in "cell space" where the period matches the texture dimensions so wrapping with
    // sampler repeat is seamless (no hard seam/wall at the edges).
    let p = uv * vec2<f32>(texture_size) * fbm_params.frequency;

    // R: coverage (macro placement)
    let coverage = fbm_2d(p);

    // G/B: "type" controls used to shape the vertical profile (NUBIS-style top/bottom types).
    // Use low-frequency tileable noise so types vary slowly over the world.
    let bottom_type = clamp(value_noise_2d(p * 0.12 + vec2(37.0, 11.0)) * 0.5 + 0.5, 0.0, 1.0);
    let top_type = clamp(value_noise_2d(p * 0.09 + vec2(5.0, 71.0)) * 0.5 + 0.5, 0.0, 1.0);

    // A: detail noise (used for erosion / "up-rez").
    // Higher frequency, but still tileable due to wrapped lattice.
    let detail = clamp(fbm_2d(p * 4.0 + vec2(13.0, 53.0)), 0.0, 1.0);

    textureStore(
        noise_texture_out,
        vec2<i32>(gid.xy),
        vec4(coverage, bottom_type, top_type, detail),
    );
}

// Wrap lattice coordinates to a 2D period (used to generate *tileable* value noise).
fn wrap2(v: vec2<f32>, size: vec2<i32>) -> vec2<f32> {
    let ix = i32(v.x);
    let iy = i32(v.y);
    let wx = ((ix % size.x) + size.x) % size.x;
    let wy = ((iy % size.y) + size.y) % size.y;
    return vec2<f32>(f32(wx), f32(wy));
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
    // Period for seamless wrapping: texture dimensions.
    // This makes noise tile cleanly when sampled with AddressMode::Repeat.
    let size = vec2<i32>(textureDimensions(noise_texture_out));
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);

    let a = hash_2d(wrap2(i + vec2(0.0, 0.0), size));
    let b = hash_2d(wrap2(i + vec2(1.0, 0.0), size));
    let c = hash_2d(wrap2(i + vec2(0.0, 1.0), size));
    let d = hash_2d(wrap2(i + vec2(1.0, 1.0), size));

    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y) * 2.0 - 1.0;
}

fn hash_2d(p: vec2<f32>) -> f32 {
    let p3 = fract(vec3(p.x, p.y, p.x) * 0.1031);
    let s = dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * (p3.z + s));
}

