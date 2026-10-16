// Tileable 3D cloud detail noise after Nubis Cubed (Schneider, SIGGRAPH 2023 Advances, "Voxel
// Cloud Noise"; recipe from Guerrilla's Nubis Noise Generator): 128³ RGBA =
//   R/G: low/high-frequency "curly alligator" (inverted alligator turbulence, curl-distorted)
//   B/A: low/high-frequency alligator (billows: inverted cells with high-frequency rims).
// Alligator noise: SideFX HDK reference (per-cell random feature point with a random weight
// times smoothstep(1 - d); value = largest minus second-largest weighted contribution).
// Lattices wrap at integer periods so the texture tiles exactly (the generator blends borders).
@group(0) @binding(13) var noise_texture_out: texture_storage_3d<rgba16float, write>;

struct Params {
    base_frequency: u32,
    // 0: raw turbulence sums, 2: raw rim sums (calibration), 1: final channels.
    mode: u32,
    _pad1: f32,
    _pad2: f32,
}

@group(0) @binding(14) var<uniform> params: Params;

fn pcg3d(v_in: vec3<u32>) -> vec3<u32> {
    var v = v_in * 1664525u + 1013904223u;
    v.x += v.y * v.z; v.y += v.z * v.x; v.z += v.x * v.y;
    v = v ^ (v >> vec3(16u));
    v.x += v.y * v.z; v.y += v.z * v.x; v.z += v.x * v.y;
    return v;
}

fn hash4(c: vec3<i32>, seed: u32) -> vec4<f32> {
    let h = pcg3d(vec3<u32>(c) + vec3(seed * 1013u, seed * 7919u, seed * 104729u));
    let h2 = pcg3d(h ^ vec3(0x9e3779b9u));
    return vec4(vec3<f32>(h), f32(h2.x)) * (1.0 / 4294967295.0);
}

fn wrapi(c: vec3<i32>, period: i32) -> vec3<i32> {
    return ((c % period) + period) % period;
}

/// SideFX alligator noise, periodic with `period` cells.
fn alligator(p: vec3<f32>, period: i32, seed: u32) -> f32 {
    let ip = vec3<i32>(floor(p));
    let fp = p - floor(p);
    var m1 = 0.0;
    var m2 = 0.0;
    var count = 0u;
    for (var z = -1; z <= 1; z++) {
        for (var y = -1; y <= 1; y++) {
            for (var x = -1; x <= 1; x++) {
                let o = vec3(x, y, z);
                let h = hash4(wrapi(ip + o, period), seed);
                let d = length(fp - (h.xyz + vec3<f32>(o)));
                if (d < 1.0) {
                    let t = 1.0 - d;
                    let v = h.w * t * t * (3.0 - 2.0 * t);
                    count += 1u;
                    if (v > m1) { m2 = m1; m1 = v; } else if (v > m2) { m2 = v; }
                }
            }
        }
    }
    return select(m1, m1 - m2, count > 1u);
}

/// Turbulence: octaves at doubling frequency, amplitude rough^i (Houdini turbulent noise).
fn alligator_turb(p: vec3<f32>, freq: i32, rough: f32, octaves: u32, seed: u32) -> f32 {
    var sum = 0.0;
    var amp = 1.0;
    var f = freq;
    for (var i = 0u; i < octaves; i++) {
        sum += amp * alligator(p * f32(f), f, seed + i * 31u);
        amp *= rough;
        f *= 2;
    }
    return sum;
}

fn fade(t: vec3<f32>) -> vec3<f32> {
    return t * t * t * (t * (t * 6.0 - 15.0) + 10.0);
}

/// Periodic gradient (Perlin) noise.
fn perlin(p: vec3<f32>, period: i32, seed: u32) -> f32 {
    let ip = vec3<i32>(floor(p));
    let fp = p - floor(p);
    let u = fade(fp);
    var n: array<f32, 8>;
    for (var k = 0; k < 8; k++) {
        let o = vec3(k & 1, (k >> 1) & 1, (k >> 2) & 1);
        let g = normalize(hash4(wrapi(ip + o, period), seed).xyz * 2.0 - 1.0 + 1e-6);
        n[k] = dot(g, fp - vec3<f32>(o));
    }
    let x00 = mix(n[0], n[1], u.x);
    let x10 = mix(n[2], n[3], u.x);
    let x01 = mix(n[4], n[5], u.x);
    let x11 = mix(n[6], n[7], u.x);
    return mix(mix(x00, x10, u.y), mix(x01, x11, u.y), u.z);
}

fn perlin_turb(p: vec3<f32>, freq: i32, octaves: u32, seed: u32) -> f32 {
    var sum = 0.0;
    var amp = 1.0;
    var f = freq;
    for (var i = 0u; i < octaves; i++) {
        sum += amp * perlin(p * f32(f), f, seed + i * 17u);
        amp *= 0.5;
        f *= 2;
    }
    return sum;
}

/// Curl of a vector Perlin potential (3 octaves), derivative w.r.t. the unit-cube position.
fn curl(p: vec3<f32>, freq: i32, seed: u32) -> vec3<f32> {
    let e = 1e-3;
    let dx = vec3(e, 0.0, 0.0);
    let dy = vec3(0.0, e, 0.0);
    let dz = vec3(0.0, 0.0, e);
    let a = seed;
    let b = seed + 101u;
    let c = seed + 211u;
    let dPz_dy = perlin_turb(p + dy, freq, 3u, c) - perlin_turb(p - dy, freq, 3u, c);
    let dPy_dz = perlin_turb(p + dz, freq, 3u, b) - perlin_turb(p - dz, freq, 3u, b);
    let dPx_dz = perlin_turb(p + dz, freq, 3u, a) - perlin_turb(p - dz, freq, 3u, a);
    let dPz_dx = perlin_turb(p + dx, freq, 3u, c) - perlin_turb(p - dx, freq, 3u, c);
    let dPy_dx = perlin_turb(p + dx, freq, 3u, b) - perlin_turb(p - dx, freq, 3u, b);
    let dPx_dy = perlin_turb(p + dy, freq, 3u, a) - perlin_turb(p - dy, freq, 3u, a);
    return vec3(dPz_dy - dPy_dz, dPx_dz - dPz_dx, dPy_dx - dPx_dy) / (2.0 * e);
}

fn fit01(v: f32, lo: f32, hi: f32) -> f32 {
    return saturate((v - lo) / (hi - lo));
}

// Generator constants (Nubis Noise Generator): curl displacement 0.01 / 0.005 of the tile, curl
// frequency 3 / 5, alligator roughness 0.608 (curly) / 0.5 (billows), 5 octaves, rim detail at
// frequency 26.5 -> 26 with attenuation 0.5 into [0.214, 0]. RAW_* are the per-channel input
// ranges of our alligator turbulence matched to the histograms of the generator's reference
// output (quantile RMS 0.002-0.008; .clouds-handoff/validation/noise/nubis_noise_calibrate.py).
const RAW_CURLY_LO: vec2<f32> = vec2(-1.1813, -1.0681);
const RAW_CURLY_HI: vec2<f32> = vec2(1.8495, 1.7587);
const RAW_ALLI_LO: vec2<f32> = vec2(0.0145, 0.0283);
const RAW_ALLI_HI: vec2<f32> = vec2(1.8773, 1.6494);
const RAW_RIM_HI: vec2<f32> = vec2(1.5748, 1.5946);

@compute
@workgroup_size(4, 4, 4)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = textureDimensions(noise_texture_out);
    if (gid.x >= dims.x || gid.y >= dims.y || gid.z >= dims.z) {
        return;
    }
    let p = (vec3<f32>(gid) + 0.5) / vec3<f32>(dims);

    // Curly alligator: alligator turbulence at a curl-displaced position.
    let q_lo = p + 0.01 * curl(p, 3, 7u);
    let q_hi = p + 0.005 * curl(p, 5, 13u);
    let s_r = alligator_turb(q_lo, 2, 0.608, 5u, 101u);
    let s_g = alligator_turb(q_hi, 4, 0.608, 5u, 202u);
    // Billows: alligator turbulence plus high-frequency rims.
    let s_b = alligator_turb(p, 3, 0.5, 5u, 303u);
    let s_a = alligator_turb(p, 5, 0.5, 5u, 404u);
    let rim_b = sqrt(max(alligator_turb(p, 26, 0.5, 5u, 505u), 0.0));
    let rim_a = sqrt(max(alligator_turb(p, 26, 0.5, 5u, 606u), 0.0));

    if (params.mode == 0u) {
        textureStore(noise_texture_out, vec3<i32>(gid), vec4(s_r, s_g, s_b, s_a));
        return;
    }
    if (params.mode == 2u) {
        textureStore(noise_texture_out, vec3<i32>(gid), vec4(rim_b, rim_a, 0.0, 0.0));
        return;
    }
    // R/G: fit(raw -> 1..0) (inverted), fit(0.3..0.7 -> 1..0), pow 4.
    let r = pow(1.0 - fit01(1.0 - fit01(s_r, RAW_CURLY_LO.x, RAW_CURLY_HI.x), 0.3, 0.7), 4.0);
    let g = pow(1.0 - fit01(1.0 - fit01(s_g, RAW_CURLY_LO.y, RAW_CURLY_HI.y), 0.3, 0.7), 4.0);
    // B/A: max((1 - fit(raw))^6, fit(rim -> 0.214..0)).
    let b = max(pow(1.0 - fit01(s_b, RAW_ALLI_LO.x, RAW_ALLI_HI.x), 6.0), 0.214 * (1.0 - fit01(rim_b, 0.0, RAW_RIM_HI.x)));
    let a = max(pow(1.0 - fit01(s_a, RAW_ALLI_LO.y, RAW_ALLI_HI.y), 6.0), 0.214 * (1.0 - fit01(rim_a, 0.0, RAW_RIM_HI.y)));
    textureStore(noise_texture_out, vec3<i32>(gid), vec4(r, g, b, a));
}
