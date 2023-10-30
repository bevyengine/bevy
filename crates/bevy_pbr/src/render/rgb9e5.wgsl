#define_import_path bevy_pbr::rgb9e5

const RGB9E5_EXPONENT_BITS        = 5u;
const RGB9E5_MANTISSA_BITS        = 9;
const RGB9E5_MANTISSA_BITSU       = 9u;
const RGB9E5_EXP_BIAS             = 15;
const RGB9E5_MAX_VALID_BIASED_EXP = 31u;

//#define MAX_RGB9E5_EXP               (RGB9E5_MAX_VALID_BIASED_EXP - RGB9E5_EXP_BIAS)
//#define RGB9E5_MANTISSA_VALUES       (1<<RGB9E5_MANTISSA_BITS)
//#define MAX_RGB9E5_MANTISSA          (RGB9E5_MANTISSA_VALUES-1)
//#define MAX_RGB9E5                   ((f32(MAX_RGB9E5_MANTISSA))/RGB9E5_MANTISSA_VALUES * (1<<MAX_RGB9E5_EXP))
//#define EPSILON_RGB9E5_              ((1.0/RGB9E5_MANTISSA_VALUES) / (1<<RGB9E5_EXP_BIAS))

const MAX_RGB9E5_EXP              = 16u;
const RGB9E5_MANTISSA_VALUES      = 512;
const MAX_RGB9E5_MANTISSA         = 511;
const MAX_RGB9E5_MANTISSAU        = 511u;
const MAX_RGB9E5_                 = 65408.0;
const EPSILON_RGB9E5_             = 0.000000059604645;

fn floor_log2_(x: f32) -> i32 {
    let f = bitcast<u32>(x);
    let biasedexponent = (f & 0x7F800000u) >> 23u;
    return i32(biasedexponent) - 127;
}

// https://www.khronos.org/registry/OpenGL/extensions/EXT/EXT_texture_shared_exponent.txt
fn vec3_to_rgb9e5_(rgb_in: vec3<f32>) -> u32 {
    let rgb = clamp(rgb_in, vec3(0.0), vec3(MAX_RGB9E5_));

    let maxrgb = max(rgb.r, max(rgb.g, rgb.b));
    var exp_shared = max(-RGB9E5_EXP_BIAS - 1, floor_log2_(maxrgb)) + 1 + RGB9E5_EXP_BIAS;
    var denom = exp2(f32(exp_shared - RGB9E5_EXP_BIAS - RGB9E5_MANTISSA_BITS));

    let maxm = i32(floor(maxrgb / denom + 0.5));
    if (maxm == RGB9E5_MANTISSA_VALUES) {
        denom *= 2.0;
        exp_shared += 1;
    }

    let n = vec3<u32>(floor(rgb / denom + 0.5));
    
    return (u32(exp_shared) << 27u) | (n.b << 18u) | (n.g << 9u) | (n.r << 0u);
}

// Builtin extractBits() is not working on WEBGL or DX12
// DX12: HLSL: Unimplemented("write_expr_math ExtractBits")
fn extract_bits(value: u32, offset: u32, bits: u32) -> u32 {
    let mask = (1u << bits) - 1u;
    return (value >> offset) & mask;
}

fn rgb9e5_to_vec3_(v: u32) -> vec3<f32> {
    let exponent = i32(extract_bits(v, 27u, RGB9E5_EXPONENT_BITS)) - RGB9E5_EXP_BIAS - RGB9E5_MANTISSA_BITS;
    let scale = exp2(f32(exponent));

    return vec3(
        f32(extract_bits(v, 0u, RGB9E5_MANTISSA_BITSU)),
        f32(extract_bits(v, 9u, RGB9E5_MANTISSA_BITSU)),
        f32(extract_bits(v, 18u, RGB9E5_MANTISSA_BITSU))
    ) * scale;
}
