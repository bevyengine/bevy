#define_import_path bevy_pbr::utils

#import bevy_pbr::rgb9e5

const PI: f32 = 3.141592653589793;      // π
const PI_2: f32 = 6.283185307179586;    // 2π
const HALF_PI: f32 = 1.57079632679;     // π/2
const FRAC_PI_3: f32 = 1.0471975512;    // π/3
const E: f32 = 2.718281828459045;       // exp(1)

// Converts HSV to RGB.
//
// Input: H ∈ [0, 2π), S ∈ [0, 1], V ∈ [0, 1].
// Output: R ∈ [0, 1], G ∈ [0, 1], B ∈ [0, 1].
//
// <https://en.wikipedia.org/wiki/HSL_and_HSV#HSV_to_RGB_alternative>
fn hsv_to_rgb(hsv: vec3<f32>) -> vec3<f32> {
    let n = vec3(5.0, 3.0, 1.0);
    let k = (n + hsv.x / FRAC_PI_3) % 6.0;
    return hsv.z - hsv.z * hsv.y * max(vec3(0.0), min(k, min(4.0 - k, vec3(1.0))));
}

// Converts RGB to HSV.
//
// Input: R ∈ [0, 1], G ∈ [0, 1], B ∈ [0, 1].
// Output: H ∈ [0, 2π), S ∈ [0, 1], V ∈ [0, 1].
//
// <https://en.wikipedia.org/wiki/HSL_and_HSV#From_RGB>
fn rgb_to_hsv(rgb: vec3<f32>) -> vec3<f32> {
    let x_max = max(rgb.r, max(rgb.g, rgb.b));  // i.e. V
    let x_min = min(rgb.r, min(rgb.g, rgb.b));
    let c = x_max - x_min;  // chroma

    var swizzle = vec3<f32>(0.0);
    if (x_max == rgb.r) {
        swizzle = vec3(rgb.gb, 0.0);
    } else if (x_max == rgb.g) {
        swizzle = vec3(rgb.br, 2.0);
    } else {
        swizzle = vec3(rgb.rg, 4.0);
    }

    let h = FRAC_PI_3 * (((swizzle.x - swizzle.y) / c + swizzle.z) % 6.0);

    // Avoid division by zero.
    var s = 0.0;
    if (x_max > 0.0) {
        s = c / x_max;
    }

    return vec3(h, s, x_max);
}

// Generates a random u32 in range [0, u32::MAX].
//
// `state` is a mutable reference to a u32 used as the seed.
//
// Values are generated via "white noise", with no correlation between values.
// In shaders, you often want spatial and/or temporal correlation. Use a different RNG method for these use cases.
//
// https://www.pcg-random.org
// https://www.reedbeta.com/blog/hash-functions-for-gpu-rendering
fn rand_u(state: ptr<function, u32>) -> u32 {
    *state = *state * 747796405u + 2891336453u;
    let word = ((*state >> ((*state >> 28u) + 4u)) ^ *state) * 277803737u;
    return (word >> 22u) ^ word;
}

// Generates a random f32 in range [0, 1.0].
fn rand_f(state: ptr<function, u32>) -> f32 {
    *state = *state * 747796405u + 2891336453u;
    let word = ((*state >> ((*state >> 28u) + 4u)) ^ *state) * 277803737u;
    return f32((word >> 22u) ^ word) * bitcast<f32>(0x2f800004u);
}

// Generates a random vec2<f32> where each value is in range [0, 1.0].
fn rand_vec2f(state: ptr<function, u32>) -> vec2<f32> {
    return vec2(rand_f(state), rand_f(state));
}

// Generates a random u32 in range [0, n).
fn rand_range_u(n: u32, state: ptr<function, u32>) -> u32 {
    return rand_u(state) % n;
}

// returns the (0-1, 0-1) position within the given viewport for the current buffer coords .
// buffer coords can be obtained from `@builtin(position).xy`.
// the view uniform struct contains the current camera viewport in `view.viewport`.
// topleft = 0,0
fn coords_to_viewport_uv(position: vec2<f32>, viewport: vec4<f32>) -> vec2<f32> {
    return (position - viewport.xy) / viewport.zw;
}

// https://jcgt.org/published/0003/02/01/paper.pdf

// For encoding normals or unit direction vectors as octahedral coordinates.
fn octahedral_encode(v: vec3<f32>) -> vec2<f32> {
    var n = v / (abs(v.x) + abs(v.y) + abs(v.z));
    let octahedral_wrap = (1.0 - abs(n.yx)) * select(vec2(-1.0), vec2(1.0), n.xy > vec2f(0.0));
    let n_xy = select(octahedral_wrap, n.xy, n.z >= 0.0);
    return n_xy * 0.5 + 0.5;
}

// For decoding normals or unit direction vectors from octahedral coordinates.
fn octahedral_decode(v: vec2<f32>) -> vec3<f32> {
    let f = v * 2.0 - 1.0;
    var n = vec3(f.xy, 1.0 - abs(f.x) - abs(f.y));
    let t = saturate(-n.z);
    let w = select(vec2(t), vec2(-t), n.xy >= vec2(0.0));
    n = vec3(n.xy + w, n.z);
    return normalize(n);
}

// https://blog.demofox.org/2022/01/01/interleaved-gradient-noise-a-different-kind-of-low-discrepancy-sequence
fn interleaved_gradient_noise(pixel_coordinates: vec2<f32>, frame: u32) -> f32 {
    let xy = pixel_coordinates + 5.588238 * f32(frame % 64u);
    return fract(52.9829189 * fract(0.06711056 * xy.x + 0.00583715 * xy.y));
}

// https://www.iryoku.com/next-generation-post-processing-in-call-of-duty-advanced-warfare (slides 120-135)
// TODO: Use an array here instead of a bunch of constants, once arrays work properly under DX12.
// NOTE: The names have a final underscore to avoid the following error:
// `Composable module identifiers must not require substitution according to naga writeback rules`
const SPIRAL_OFFSET_0_ = vec2<f32>(-0.7071,  0.7071);
const SPIRAL_OFFSET_1_ = vec2<f32>(-0.0000, -0.8750);
const SPIRAL_OFFSET_2_ = vec2<f32>( 0.5303,  0.5303);
const SPIRAL_OFFSET_3_ = vec2<f32>(-0.6250, -0.0000);
const SPIRAL_OFFSET_4_ = vec2<f32>( 0.3536, -0.3536);
const SPIRAL_OFFSET_5_ = vec2<f32>(-0.0000,  0.3750);
const SPIRAL_OFFSET_6_ = vec2<f32>(-0.1768, -0.1768);
const SPIRAL_OFFSET_7_ = vec2<f32>( 0.1250,  0.0000);
