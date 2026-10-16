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
    // Tileable gradient (Perlin) FBM with a light domain warp: rounded, non-axis-aligned cells.
    // The previous value noise (weak float hash on small lattice ints) gave boxy coverage edges,
    // and `value_noise_tiled(uv * 4, 4)` repeated the cloud-type pattern 4x4 with sharp ramps,
    // which stepped cloud heights between cells (blocky slabs / dark rectangular patches).
    let warp = vec2(perlin_fbm_tiled(uv + vec2(0.31, 0.77), 8, 3), perlin_fbm_tiled(uv + vec2(0.63, 0.19), 8, 3)) - 0.5;
    let coverage_raw = perlin_fbm_tiled(uv + warp * 0.03, 8, 4);
    // Cumulus-style coverage: carve out clear sky between distinct cells.
    let coverage = smoothstep(0.40, 0.75, coverage_raw);
    // Cloud type varies smoothly over ~8 km.
    let cloud_type = clamp(smoothstep(0.3, 0.7, perlin_fbm_tiled(uv + vec2(0.37, 0.11), 4, 2)) * 0.6 + 0.4, 0.0, 1.0);
    // Density scale (Nubis NVDF): 1 = full density. The procedural map does not modulate it
    // (LWC sets the physical magnitude); a painted weather map can.
    let density_scale = 1.0;
    // Mesoscale cloud fraction (A): sampled at 4x the tile (clouds.wesl `sample_weather`), so its
    // periods of 2/4/8 cells give 64/32/16 km structure at the default 32 km tile. Cumulus fields
    // organise at 10-100 km (cloudy clusters and clear gaps); a single 4 km fBm with a fixed
    // threshold was statistically uniform beyond 4 km, so low sun found no clear path to light
    // cloud bases.
    let spare = perlin_fbm_tiled(uv + vec2(0.71, 0.53), 2, 3);

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

fn pcg2(v: vec2<u32>) -> u32 {
    var x = v.x * 1664525u + v.y * 1013904223u + 0x9e3779b9u;
    x ^= x >> 16u; x *= 0x7feb352du; x ^= x >> 15u; x *= 0x846ca68bu; x ^= x >> 16u;
    return x;
}

fn grad_tiled(i: vec2<i32>, period: i32, seed: u32) -> vec2<f32> {
    let w = ((i % vec2(period)) + vec2(period)) % vec2(period);
    let h = pcg2(vec2<u32>(u32(w.x), u32(w.y) ^ (seed * 0x27d4eb2du)));
    let a = f32(h & 0xffffu) / 65535.0 * 6.2831853;
    return vec2(cos(a), sin(a));
}

// Tileable 2D gradient noise in about [-0.7, 0.7]; `period` lattice cells across the unit square.
fn perlin_tiled(uvc: vec2<f32>, period: i32, seed: u32) -> f32 {
    let q = uvc * f32(period);
    let i = vec2<i32>(floor(q));
    let f = fract(q);
    let u = f * f * f * (f * (f * 6.0 - 15.0) + 10.0);
    let a = dot(grad_tiled(i, period, seed), f);
    let b = dot(grad_tiled(i + vec2(1, 0), period, seed), f - vec2(1.0, 0.0));
    let c = dot(grad_tiled(i + vec2(0, 1), period, seed), f - vec2(0.0, 1.0));
    let d = dot(grad_tiled(i + vec2(1, 1), period, seed), f - vec2(1.0, 1.0));
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

// Tileable Perlin FBM remapped to roughly [0, 1].
fn perlin_fbm_tiled(uvc: vec2<f32>, period: i32, octaves: u32) -> f32 {
    var value = 0.0;
    var amp = 0.5;
    var norm = 0.0;
    var per = period;
    for (var o = 0u; o < octaves; o++) {
        value += amp * perlin_tiled(uvc, per, o + u32(period) * 7u);
        norm += amp;
        per *= 2;
        amp *= 0.5;
    }
    return clamp(value / norm * 0.9 + 0.5, 0.0, 1.0);
}
