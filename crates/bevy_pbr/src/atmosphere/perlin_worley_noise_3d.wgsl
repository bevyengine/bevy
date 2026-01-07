// Tileable 3D Perlin–Worley Noise Generator
// Writes RGBA channels:
// - R: Perlin–Worley (remapped billowy Perlin by low-frequency Worley FBM)
// - G: Worley FBM (freq)
// - B: Worley FBM (2*freq)
// - A: Worley FBM (4*freq)

@group(0) @binding(13) var noise_texture_out: texture_storage_3d<rgba16float, write>;

struct Params {
    base_frequency: u32,
    perlin_octaves: u32,
    z_offset: f32,
    _pad0: f32,
}

@group(0) @binding(14) var<uniform> params: Params;

const UI0: u32 = 1597334673u;
const UI1: u32 = 3812015801u;
const UI2: vec2<u32> = vec2<u32>(UI0, UI1);
const UI3: vec3<u32> = vec3<u32>(UI0, UI1, 2798796415u);
const UIF: f32 = 1.0 / 4294967295.0;

fn hash33(p: vec3<f32>) -> vec3<f32> {
    // Hash by David_Hoskins, adapted to WGSL.
    let ip = vec3<i32>(p);
    var q: vec3<u32> = vec3<u32>(u32(ip.x), u32(ip.y), u32(ip.z)) * UI3;
    q = (q.xxx ^ q.yyy ^ q.zzz) * UI3;
    return -1.0 + 2.0 * vec3<f32>(q) * UIF;
}

fn remap(x: f32, a: f32, b: f32, c: f32, d: f32) -> f32 {
    let t = clamp((x - a) / max(1e-6, b - a), 0.0, 1.0);
    return mix(c, d, t);
}

fn imod(a: i32, m: i32) -> i32 {
    // positive modulus
    let r = a % m;
    return select(r + m, r, r >= 0);
}

fn wrap3(p: vec3<i32>, m: i32) -> vec3<f32> {
    return vec3<f32>(
        f32(imod(p.x, m)),
        f32(imod(p.y, m)),
        f32(imod(p.z, m)),
    );
}

// Gradient noise by iq (modified to be tileable)
fn gradientNoise(x: vec3<f32>, freq: u32) -> f32 {
    let period: i32 = max(1, i32(freq));
    // grid
    let p = floor(x);
    let w = fract(x);

    // quintic interpolant
    let u = w * w * w * (w * (w * 6.0 - 15.0) + 10.0);

    // gradients (wrapped lattice)
    let pi = vec3<i32>(p);
    let ga = hash33(wrap3(pi + vec3<i32>(0, 0, 0), period));
    let gb = hash33(wrap3(pi + vec3<i32>(1, 0, 0), period));
    let gc = hash33(wrap3(pi + vec3<i32>(0, 1, 0), period));
    let gd = hash33(wrap3(pi + vec3<i32>(1, 1, 0), period));
    let ge = hash33(wrap3(pi + vec3<i32>(0, 0, 1), period));
    let gf = hash33(wrap3(pi + vec3<i32>(1, 0, 1), period));
    let gg = hash33(wrap3(pi + vec3<i32>(0, 1, 1), period));
    let gh = hash33(wrap3(pi + vec3<i32>(1, 1, 1), period));

    // projections
    let va = dot(ga, w - vec3<f32>(0.0, 0.0, 0.0));
    let vb = dot(gb, w - vec3<f32>(1.0, 0.0, 0.0));
    let vc = dot(gc, w - vec3<f32>(0.0, 1.0, 0.0));
    let vd = dot(gd, w - vec3<f32>(1.0, 1.0, 0.0));
    let ve = dot(ge, w - vec3<f32>(0.0, 0.0, 1.0));
    let vf = dot(gf, w - vec3<f32>(1.0, 0.0, 1.0));
    let vg = dot(gg, w - vec3<f32>(0.0, 1.0, 1.0));
    let vh = dot(gh, w - vec3<f32>(1.0, 1.0, 1.0));

    // interpolation (expanded trilinear)
    return va +
        u.x * (vb - va) +
        u.y * (vc - va) +
        u.z * (ve - va) +
        u.x * u.y * (va - vb - vc + vd) +
        u.y * u.z * (va - vc - ve + vg) +
        u.z * u.x * (va - vb - ve + vf) +
        u.x * u.y * u.z * (-va + vb + vc - vd + ve - vf - vg + vh);
}

// Tileable 3D Worley noise (inverted)
fn worleyNoise(uv: vec3<f32>, freq: u32) -> f32 {
    let period: i32 = max(1, i32(freq));
    let id = floor(uv);
    let p = fract(uv);

    var minDist = 1e9;
    let idi = vec3<i32>(id);

    for (var xo: i32 = -1; xo <= 1; xo++) {
        for (var yo: i32 = -1; yo <= 1; yo++) {
            for (var zo: i32 = -1; zo <= 1; zo++) {
                let offseti = vec3<i32>(xo, yo, zo);
                let h = hash33(wrap3(idi + offseti, period)) * 0.5 + 0.5;
                let hp = h + vec3<f32>(f32(xo), f32(yo), f32(zo));
                let d = p - hp;
                minDist = min(minDist, dot(d, d));
            }
        }
    }

    return 1.0 - minDist;
}

// Tileable Worley FBM inspired by Andrew Schneider's Real-Time Volumetric Cloudscapes (GPU Pro 7)
fn worleyFbm(p: vec3<f32>, freq: u32) -> f32 {
    let f0 = max(1u, freq);
    let f1 = max(1u, f0 * 2u);
    let f2 = max(1u, f0 * 4u);
    return worleyNoise(p * f32(f0), f0) * 0.625 +
        worleyNoise(p * f32(f1), f1) * 0.25 +
        worleyNoise(p * f32(f2), f2) * 0.125;
}

// FBM for Perlin/gradient noise (iq-inspired) — returns roughly [-1,1]
fn perlinfbm(p: vec3<f32>, freq: u32, octaves: u32) -> f32 {
    let G = exp2(-0.85);
    var amp = 1.0;
    var f = max(1u, freq);
    var n = 0.0;
    for (var i: u32 = 0u; i < octaves; i++) {
        n += amp * gradientNoise(p * f32(f), f);
        f *= 2u;
        amp *= G;
    }
    return n;
}

@compute
@workgroup_size(4, 4, 4)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = textureDimensions(noise_texture_out);
    if (gid.x >= dims.x || gid.y >= dims.y || gid.z >= dims.z) {
        return;
    }

    // Normalized coordinates in [0,1)
    let uvw = (vec3<f32>(gid) + 0.5) / vec3<f32>(dims);
    let p = vec3<f32>(uvw.xy, fract(uvw.z + params.z_offset));

    let base = max(1u, params.base_frequency);

    // Billowy Perlin FBM in [0,1]
    var pfbm = mix(1.0, perlinfbm(p, base, params.perlin_octaves), 0.5);
    pfbm = abs(pfbm * 2.0 - 1.0);
    pfbm = clamp(pfbm, 0.0, 1.0);

    // Worley FBMs at 3 frequencies
    let w0 = clamp(worleyFbm(p, base), 0.0, 1.0);
    let w1 = clamp(worleyFbm(p, base * 2u), 0.0, 1.0);
    let w2 = clamp(worleyFbm(p, base * 4u), 0.0, 1.0);

    // Perlin–Worley: remap Perlin by low-frequency Worley
    let pw = remap(pfbm, 0.0, 1.0, w0, 1.0);

    textureStore(
        noise_texture_out,
        vec3<i32>(gid),
        vec4<f32>(pw, w0, w1, w2),
    );
}


