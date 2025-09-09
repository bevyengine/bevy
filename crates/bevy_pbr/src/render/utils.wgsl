#define_import_path bevy_pbr::utils

#import bevy_pbr::rgb9e5
#import bevy_render::maths::{PI, PI_2, orthonormalize}

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
    return octahedral_decode_signed(f);
}

// Like octahedral_decode, but for input in [-1, 1] instead of [0, 1].
fn octahedral_decode_signed(v: vec2<f32>) -> vec3<f32> {
    var n = vec3(v.xy, 1.0 - abs(v.x) - abs(v.y));
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

// Hammersley sequence for quasi-random points
fn hammersley_2d(i: u32, n: u32) -> vec2f {
    let inv_n = 1.0 / f32(n);
    let vdc = f32(reverseBits(i)) * 2.3283064365386963e-10; // 1/2^32
    return vec2f(f32(i) * inv_n, vdc);
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

// https://www.realtimerendering.com/raytracinggems/unofficial_RayTracingGems_v1.9.pdf#0004286901.INDD%3ASec28%3A303
fn sample_cosine_hemisphere(normal: vec3<f32>, rng: ptr<function, u32>) -> vec3<f32> {
    let cos_theta = 1.0 - 2.0 * rand_f(rng);
    let phi = PI_2 * rand_f(rng);
    let sin_theta = sqrt(max(1.0 - cos_theta * cos_theta, 0.0));
    let x = normal.x + sin_theta * cos(phi);
    let y = normal.y + sin_theta * sin(phi);
    let z = normal.z + cos_theta;
    return vec3(x, y, z);
}
// https://www.pbr-book.org/3ed-2018/Monte_Carlo_Integration/2D_Sampling_with_Multidimensional_Transformations#UniformlySamplingaHemisphere
fn sample_uniform_hemisphere(normal: vec3<f32>, rng: ptr<function, u32>) -> vec3<f32> {
    let cos_theta = rand_f(rng);
    let phi = PI_2 * rand_f(rng);
    let sin_theta = sqrt(max(1.0 - cos_theta * cos_theta, 0.0));
    let x = sin_theta * cos(phi);
    let y = sin_theta * sin(phi);
    let z = cos_theta;
    return orthonormalize(normal) * vec3(x, y, z);
}

fn uniform_hemisphere_inverse_pdf() -> f32 {
    return PI_2;
}

// https://www.realtimerendering.com/raytracinggems/unofficial_RayTracingGems_v1.9.pdf#0004286901.INDD%3ASec19%3A294
fn sample_disk(disk_radius: f32, rng: ptr<function, u32>) -> vec2<f32> {
    let ab = 2.0 * rand_vec2f(rng) - 1.0;
    let a = ab.x;
    var b = ab.y;
    if (b == 0.0) { b = 1.0; }

    var phi: f32;
    var r: f32;
    if (a * a > b * b) {
        r = disk_radius * a;
        phi = (PI / 4.0) * (b / a);
    } else {
        r = disk_radius * b;
        phi = (PI / 2.0) - (PI / 4.0) * (a / b);
    }

    let x = r * cos(phi);
    let y = r * sin(phi);
    return vec2(x, y);
}

// Convert UV and face index to direction vector
fn sample_cube_dir(uv: vec2f, face: u32) -> vec3f {
    // Convert from [0,1] to [-1,1]
    let uvc = 2.0 * uv - 1.0;

    // Generate direction based on the cube face
    var dir: vec3f;
    switch(face) {
        case 0u: { dir = vec3f( 1.0,  -uvc.y, -uvc.x); } // +X
        case 1u: { dir = vec3f(-1.0,  -uvc.y,  uvc.x); } // -X
        case 2u: { dir = vec3f( uvc.x,  1.0,   uvc.y); } // +Y
        case 3u: { dir = vec3f( uvc.x, -1.0,  -uvc.y); } // -Y
        case 4u: { dir = vec3f( uvc.x, -uvc.y,  1.0);  } // +Z
        case 5u: { dir = vec3f(-uvc.x, -uvc.y, -1.0);  } // -Z
        default: { dir = vec3f(0.0); }
    }
    return normalize(dir);
}

// Convert direction vector to cube face UV
struct CubeUV {
    uv: vec2f,
    face: u32,
}
fn dir_to_cube_uv(dir: vec3f) -> CubeUV {
    let abs_dir = abs(dir);
    var face: u32 = 0u;
    var uv: vec2f = vec2f(0.0);

    // Find the dominant axis to determine face
    if (abs_dir.x >= abs_dir.y && abs_dir.x >= abs_dir.z) {
        // X axis is dominant
        if (dir.x > 0.0) {
            face = 0u; // +X
            uv = vec2f(-dir.z, -dir.y) / dir.x;
        } else {
            face = 1u; // -X
            uv = vec2f(dir.z, -dir.y) / abs_dir.x;
        }
    } else if (abs_dir.y >= abs_dir.x && abs_dir.y >= abs_dir.z) {
        // Y axis is dominant
        if (dir.y > 0.0) {
            face = 2u; // +Y
            uv = vec2f(dir.x, dir.z) / dir.y;
        } else {
            face = 3u; // -Y
            uv = vec2f(dir.x, -dir.z) / abs_dir.y;
        }
    } else {
        // Z axis is dominant
        if (dir.z > 0.0) {
            face = 4u; // +Z
            uv = vec2f(dir.x, -dir.y) / dir.z;
        } else {
            face = 5u; // -Z
            uv = vec2f(-dir.x, -dir.y) / abs_dir.z;
        }
    }

    // Convert from [-1,1] to [0,1]
    return CubeUV(uv * 0.5 + 0.5, face);
}
