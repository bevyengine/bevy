// Tileable 3D Worley detail noise (Nubis/HZD 32³ detail).
// R/G/B/A = Worley FBM at freq, 2x, 4x, 8x.
@group(0) @binding(13) var noise_texture_out: texture_storage_3d<rgba16float, write>;

struct Params {
    base_frequency: u32,
    _pad0: u32,
    _pad1: f32,
    _pad2: f32,
}

@group(0) @binding(14) var<uniform> params: Params;

const UI0: u32 = 1597334673u;
const UI1: u32 = 3812015801u;
const UI3: vec3<u32> = vec3<u32>(UI0, UI1, 2798796415u);
const UIF: f32 = 1.0 / 4294967295.0;

fn hash33(p: vec3<f32>) -> vec3<f32> {
    let ip = vec3<i32>(p);
    var q: vec3<u32> = vec3<u32>(u32(ip.x), u32(ip.y), u32(ip.z)) * UI3;
    q = (q.xxx ^ q.yyy ^ q.zzz) * UI3;
    return -1.0 + 2.0 * vec3<f32>(q) * UIF;
}

fn imod(a: i32, m: i32) -> i32 {
    let r = a % m;
    return select(r + m, r, r >= 0);
}

fn wrap3(p: vec3<i32>, m: i32) -> vec3<f32> {
    return vec3<f32>(f32(imod(p.x, m)), f32(imod(p.y, m)), f32(imod(p.z, m)));
}

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

fn worleyFbm(p: vec3<f32>, freq: u32) -> f32 {
    let f0 = max(1u, freq);
    let f1 = max(1u, f0 * 2u);
    let f2 = max(1u, f0 * 4u);
    return worleyNoise(p * f32(f0), f0) * 0.625 +
        worleyNoise(p * f32(f1), f1) * 0.25 +
        worleyNoise(p * f32(f2), f2) * 0.125;
}

@compute
@workgroup_size(4, 4, 4)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = textureDimensions(noise_texture_out);
    if (gid.x >= dims.x || gid.y >= dims.y || gid.z >= dims.z) {
        return;
    }
    let uvw = (vec3<f32>(gid) + 0.5) / vec3<f32>(dims);
    let base = max(1u, params.base_frequency);
    let w0 = clamp(worleyFbm(uvw, base), 0.0, 1.0);
    let w1 = clamp(worleyFbm(uvw, base * 2u), 0.0, 1.0);
    let w2 = clamp(worleyFbm(uvw, base * 4u), 0.0, 1.0);
    let w3 = clamp(worleyFbm(uvw, base * 8u), 0.0, 1.0);
    textureStore(noise_texture_out, vec3<i32>(gid), vec4<f32>(w0, w1, w2, w3));
}
