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
    // Lattice periods are expressed in cells per tile (not texels) so features have a sensible
    // world size: with the default 32 km tile, period 16 → ~2 km base coverage cells, octaves down
    // to ~250 m. (Previously the base lattice was 2 texels ≈ 125-250 m, i.e. near white noise,
    // which bilinear filtering smeared into uniform overcast haze.)
    let coverage_raw = fbm_tiled(uv, 16);
    // Cumulus-style coverage: carve out clear sky between distinct cells (~45 % cover).
    let coverage = smoothstep(0.42, 0.72, coverage_raw);
    let cloud_type = clamp(smoothstep(0.3, 0.7, value_noise_tiled(uv * 4.0 + vec2(0.37, 0.11), 4) * 0.5 + 0.5) * 0.6 + 0.4, 0.0, 1.0);
    let density_scale = fbm_tiled(uv + vec2(0.5, 0.21), 8);
    let spare = 0.0;

    textureStore(
        noise_texture_out,
        vec2<i32>(gid.xy),
        vec4(coverage, cloud_type, density_scale, spare),
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


// Value noise tileable with `period` lattice cells across the unit uv square.
fn value_noise_tiled(uvc: vec2<f32>, period: i32) -> f32 {
    let q = uvc * f32(period);
    let i = floor(q);
    let f = fract(q);
    let u = f * f * (3.0 - 2.0 * f);
    let n = vec2(period, period);
    let a = hash_2d(wrap2(i + vec2(0.0, 0.0), n));
    let b = hash_2d(wrap2(i + vec2(1.0, 0.0), n));
    let c = hash_2d(wrap2(i + vec2(0.0, 1.0), n));
    let d = hash_2d(wrap2(i + vec2(1.0, 1.0), n));
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y) * 2.0 - 1.0;
}

// Tileable FBM in [0, 1]: base `period` cells per tile, doubling per octave.
fn fbm_tiled(uvc: vec2<f32>, period: i32) -> f32 {
    var value = 0.0;
    var amp = 0.5;
    var per = period;
    for (var i = 0u; i < 4u; i++) {
        value += amp * value_noise_tiled(uvc + vec2(f32(i) * 0.137, f32(i) * 0.291), per);
        per *= 2;
        amp *= 0.5;
    }
    return clamp(value / 0.9375 * 0.5 + 0.5, 0.0, 1.0);
}
