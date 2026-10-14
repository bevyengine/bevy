// Tileable 2D curl noise (HZD/Nubis 128²). Stores (dP/dy, -dP/dx) in RG.
@group(0) @binding(13) var noise_texture_out: texture_storage_2d<rg16float, write>;

struct Params {
    base_frequency: u32,
    _pad0: u32,
    _pad1: f32,
    _pad2: f32,
}

@group(0) @binding(14) var<uniform> params: Params;

const UI0: u32 = 1597334673u;
const UI1: u32 = 3812015801u;
const UIF: f32 = 1.0 / 4294967295.0;

fn hash22(p: vec2<f32>) -> vec2<f32> {
    let ip = vec2<i32>(p);
    var q = vec2<u32>(u32(ip.x), u32(ip.y)) * vec2<u32>(UI0, UI1);
    q = (q.xx ^ q.yy) * vec2<u32>(UI0, UI1);
    return -1.0 + 2.0 * vec2<f32>(q) * UIF;
}

fn imod(a: i32, m: i32) -> i32 {
    let r = a % m;
    return select(r + m, r, r >= 0);
}

fn wrap2(p: vec2<i32>, m: i32) -> vec2<f32> {
    return vec2<f32>(f32(imod(p.x, m)), f32(imod(p.y, m)));
}

fn gradientNoise2(x: vec2<f32>, freq: u32) -> f32 {
    let period: i32 = max(1, i32(freq));
    let p = floor(x);
    let w = fract(x);
    let u = w * w * w * (w * (w * 6.0 - 15.0) + 10.0);
    let pi = vec2<i32>(p);
    let ga = hash22(wrap2(pi + vec2<i32>(0, 0), period));
    let gb = hash22(wrap2(pi + vec2<i32>(1, 0), period));
    let gc = hash22(wrap2(pi + vec2<i32>(0, 1), period));
    let gd = hash22(wrap2(pi + vec2<i32>(1, 1), period));
    let va = dot(ga, w - vec2(0.0, 0.0));
    let vb = dot(gb, w - vec2(1.0, 0.0));
    let vc = dot(gc, w - vec2(0.0, 1.0));
    let vd = dot(gd, w - vec2(1.0, 1.0));
    return va + u.x * (vb - va) + u.y * (vc - va) + u.x * u.y * (va - vb - vc + vd);
}

fn potential(uvw: vec2<f32>, freq: u32) -> f32 {
    // Two octaves of tileable gradient noise as a scalar potential.
    let f0 = max(1u, freq);
    let n0 = gradientNoise2(uvw * f32(f0), f0);
    let n1 = gradientNoise2(uvw * f32(f0 * 2u) + vec2(17.0, 9.0), f0 * 2u);
    return n0 * 0.75 + n1 * 0.25;
}

@compute
@workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = textureDimensions(noise_texture_out);
    if (gid.x >= dims.x || gid.y >= dims.y) {
        return;
    }
    let uv = (vec2<f32>(gid.xy) + 0.5) / vec2<f32>(dims);
    let freq = max(1u, params.base_frequency);
    // Finite differences in texture space; scale so magnitude ~ O(1).
    let eps = 1.0 / f32(dims.x);
    let dpy = potential(uv + vec2(0.0, eps), freq) - potential(uv - vec2(0.0, eps), freq);
    let dpx = potential(uv + vec2(eps, 0.0), freq) - potential(uv - vec2(eps, 0.0), freq);
    let curl = vec2(dpy, -dpx) / (2.0 * eps) * (1.0 / f32(freq));
    textureStore(noise_texture_out, vec2<i32>(gid.xy), vec4(curl, 0.0, 0.0));
}
