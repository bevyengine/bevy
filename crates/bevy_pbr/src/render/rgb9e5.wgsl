#define_import_path bevy_pbr::rgb9e5

// https://github.com/EmbarkStudios/kajiya/blob/d3b6ac22c5306cc9d3ea5e2d62fd872bea58d8d6/assets/shaders/inc/pack_unpack.hlsl#L101
const RGB9E5_EXPONENT_BITS        = 5u;
const RGB9E5_MANTISSA_BITS        = 9;
const RGB9E5_MANTISSA_BITSU       = 9u;
const RGB9E5_EXP_BIAS             = 15;
const RGB9E5_MAX_VALID_BIASED_EXP = 31u;

//#define MAX_RGB9E5_EXP               (RGB9E5_MAX_VALID_BIASED_EXP - RGB9E5_EXP_BIAS)
//#define RGB9E5_MANTISSA_VALUES       (1<<RGB9E5_MANTISSA_BITS)
//#define MAX_RGB9E5_MANTISSA          (RGB9E5_MANTISSA_VALUES-1)
//#define MAX_RGB9E5                   ((f32(MAX_RGB9E5_MANTISSA))/RGB9E5_MANTISSA_VALUES * (1<<MAX_RGB9E5_EXP))
//#define EPSILON_RGB9E5               ((1.0/RGB9E5_MANTISSA_VALUES) / (1<<RGB9E5_EXP_BIAS))

const MAX_RGB9E5_EXP              = 16u;
const RGB9E5_MANTISSA_VALUES      = 512u;
const MAX_RGB9E5_MANTISSA         = 511;
const MAX_RGB9E5                  = 65408.0;
const EPSILON_RGB9E5              = 0.000000059604645;

fn clamp_range_for_rgb9e5(x: f32) -> f32 {
    return clamp(x, 0.0, MAX_RGB9E5);
}

fn floor_log2(x: f32) -> i32 {
    let f = bitcast<u32>(x);
    let biasedexponent = (f & 0x7F800000u) >> 23u;
    return i32(biasedexponent) - 127;
}

// https://www.khronos.org/registry/OpenGL/extensions/EXT/EXT_texture_shared_exponent.txt
fn float3_to_rgb9e5(rgb: vec3<f32>) -> u32 {
    let rc = clamp_range_for_rgb9e5(rgb.x);
    let gc = clamp_range_for_rgb9e5(rgb.y);
    let bc = clamp_range_for_rgb9e5(rgb.z);

    let maxrgb = max(rc, max(gc, bc));
    var exp_shared = max(-RGB9E5_EXP_BIAS - 1, floor_log2(maxrgb)) + 1 + RGB9E5_EXP_BIAS;
    var denom = exp2(f32(exp_shared - RGB9E5_EXP_BIAS - RGB9E5_MANTISSA_BITS));

    let maxm = i32(floor(maxrgb / denom + 0.5));
    if (maxm == MAX_RGB9E5_MANTISSA + 1) {
        denom *= 2.0;
        exp_shared += 1;
    }

    let rm = i32(floor(rc / denom + 0.5));
    let gm = i32(floor(gc / denom + 0.5));
    let bm = i32(floor(bc / denom + 0.5));

    return (u32(rm) << (32u - 9u))
        | (u32(gm) << (32u - 9u * 2u))
        | (u32(bm) << (32u - 9u * 3u))
        | u32(exp_shared);
}

fn bitfield_extract(value: u32, offset: u32, bits: u32) -> u32 {
    let mask = (1u << bits) - 1u;
    return (value >> offset) & mask;
}

fn rgb9e5_to_float3(v: u32) -> vec3<f32> {
    let exponent = i32(bitfield_extract(v, 0u, RGB9E5_EXPONENT_BITS)) - RGB9E5_EXP_BIAS - RGB9E5_MANTISSA_BITS;
    let scale = exp2(f32(exponent));

    return vec3(
        f32(bitfield_extract(v, 32u - RGB9E5_MANTISSA_BITSU, RGB9E5_MANTISSA_BITSU)) * scale,
        f32(bitfield_extract(v, 32u - RGB9E5_MANTISSA_BITSU * 2u, RGB9E5_MANTISSA_BITSU)) * scale,
        f32(bitfield_extract(v, 32u - RGB9E5_MANTISSA_BITSU * 3u, RGB9E5_MANTISSA_BITSU)) * scale
    );
}