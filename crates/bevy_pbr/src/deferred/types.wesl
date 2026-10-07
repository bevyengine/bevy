import package::render::{
    mesh_types::MESH_FLAGS_SHADOW_RECEIVER_BIT,
    pbr_types::{STANDARD_MATERIAL_FLAGS_FOG_ENABLED_BIT, STANDARD_MATERIAL_FLAGS_UNLIT_BIT},
    rgb9e5::{vec3_to_rgb9e5_, rgb9e5_to_vec3_},
};

// Maximum of 8 bits available
const DEFERRED_FLAGS_UNLIT_BIT: u32                 = 1u << 0u;
const DEFERRED_FLAGS_FOG_ENABLED_BIT: u32           = 1u << 1u;
const DEFERRED_MESH_FLAGS_SHADOW_RECEIVER_BIT: u32  = 1u << 2u;

fn deferred_flags_from_mesh_material_flags(mesh_flags: u32, mat_flags: u32) -> u32 {
    var flags = 0u;
    flags |= u32((mesh_flags & MESH_FLAGS_SHADOW_RECEIVER_BIT) != 0u) * DEFERRED_MESH_FLAGS_SHADOW_RECEIVER_BIT;
    flags |= u32((mat_flags & STANDARD_MATERIAL_FLAGS_FOG_ENABLED_BIT) != 0u) * DEFERRED_FLAGS_FOG_ENABLED_BIT;
    flags |= u32((mat_flags & STANDARD_MATERIAL_FLAGS_UNLIT_BIT) != 0u) * DEFERRED_FLAGS_UNLIT_BIT;
    return flags;
}

fn mesh_material_flags_from_deferred_flags(deferred_flags: u32) -> vec2<u32> {
    var mat_flags = 0u;
    var mesh_flags = 0u;
    mesh_flags |= u32((deferred_flags & DEFERRED_MESH_FLAGS_SHADOW_RECEIVER_BIT) != 0u) * MESH_FLAGS_SHADOW_RECEIVER_BIT;
    mat_flags |= u32((deferred_flags & DEFERRED_FLAGS_FOG_ENABLED_BIT) != 0u) * STANDARD_MATERIAL_FLAGS_FOG_ENABLED_BIT;
    mat_flags |= u32((deferred_flags & DEFERRED_FLAGS_UNLIT_BIT) != 0u) * STANDARD_MATERIAL_FLAGS_UNLIT_BIT;
    return vec2(mesh_flags, mat_flags);
}

const U12MAXF = 4095.0;
const U16MAXF = 65535.0;
const U20MAXF = 1048575.0;

// Storing normals as oct24.
// Flags are stored in the remaining 8 bits.
// https://jcgt.org/published/0003/02/01/paper.pdf
// Could possibly go down to oct20 if the space is needed.

fn pack_24bit_normal_and_flags(octahedral_normal: vec2<f32>, flags: u32) -> u32 {
    let unorm1 = u32(saturate(octahedral_normal.x) * U12MAXF + 0.5);
    let unorm2 = u32(saturate(octahedral_normal.y) * U12MAXF + 0.5);
    return (unorm1 & 0xFFFu) | ((unorm2 & 0xFFFu) << 12u) | ((flags & 0xFFu) << 24u);
}

fn unpack_24bit_normal(packed: u32) -> vec2<f32> {
    let unorm1 = packed & 0xFFFu;
    let unorm2 = (packed >> 12u) & 0xFFFu;
    return vec2(f32(unorm1) / U12MAXF, f32(unorm2) / U12MAXF);
}

fn unpack_flags(packed: u32) -> u32 {
    return (packed >> 24u) & 0xFFu;
}

// The G-buffer stores IOR as the Fresnel ratio |(ior - 1) / (ior + 1)|, which is sqrt(F0).
// Every F0 in [0, 1] is representable: IOR 1 to 3 (F0 0 to 0.25), higher IOR, IOR between 0 and 1
// (same F0 as 1 / ior), and IOR 0 (F0 = 1).
// The ratio is quantized on a square-root scale to give more codes to the common low values.
fn ior_to_fresnel_ratio(ior: f32) -> f32 {
    // Same as `calculate_F0_dielectric`: (ior - 1) / (ior + 1), safe for IOR <= -1 and infinite IOR.
    return saturate(abs(1.0 - 2.0 / max(ior + 1.0, 1e-4)));
}

// Returns the IOR >= 1 with the given Fresnel ratio, or 0 when the ratio is 1 (F0 = 1).
fn fresnel_ratio_to_ior(ratio: f32) -> f32 {
    return select((1.0 + ratio) / max(1.0 - ratio, 1e-4), 0.0, ratio >= 1.0);
}

// Pack ior and specular strength into 4 bits.
// With so little space, specular is made binary: code 0 is specular = 0.0, and codes 1 to 15 are
// specular = 1.0 with 15 levels of IOR.
@if(WEBGL2)
fn pack_4bit_ior_specular(ior: f32, specular: f32) -> u32 {
    if specular <= 0.0 {
        return 0u;
    }
    return 1u + u32(sqrt(ior_to_fresnel_ratio(ior)) * 14.0 + 0.5);
}

@if(WEBGL2)
fn unpack_4bit_ior_specular(v: u32) -> vec2<f32> {
    if v == 0u {
        return vec2<f32>(1.5, 0.0);
    }
    let ratio_sqrt = f32(min(v, 15u) - 1u) / 14.0;
    return vec2<f32>(fresnel_ratio_to_ior(ratio_sqrt * ratio_sqrt), 1.0);
}

// Pack ior and specular strength into 8 bits. Allocation is IOR: 5 bits / specular: 3 bits
fn pack_8bit_ior_specular(ior: f32, specular: f32) -> u32 {
    let ior5 = u32(sqrt(ior_to_fresnel_ratio(ior)) * 31.0 + 0.5);
    let specular3 = u32(saturate(specular) * 7.0 + 0.5);
    return (specular3 << 5u) | ior5;
}

fn unpack_8bit_ior_specular(v: u32) -> vec2<f32> {
    let specular = f32((v >> 5u) & 0x7u) / 7.0;
    let ratio_sqrt = f32(v & 0x1Fu) / 31.0;
    return vec2<f32>(fresnel_ratio_to_ior(ratio_sqrt * ratio_sqrt), specular);
}

/// The builtin one didn't work in webgl.
/// "'unpackUnorm4x8' : no matching overloaded function found"
/// https://github.com/gfx-rs/naga/issues/2006
fn unpack_unorm4x8_(v: u32) -> vec4<f32> {
    return vec4(
        f32(v & 0xFFu),
        f32((v >> 8u) & 0xFFu),
        f32((v >> 16u) & 0xFFu),
        f32((v >> 24u) & 0xFFu)
    ) / 255.0;
}

/// 'packUnorm4x8' : no matching overloaded function found
/// https://github.com/gfx-rs/naga/issues/2006
fn pack_unorm4x8_(values: vec4<f32>) -> u32 {
    let v = vec4<u32>(saturate(values) * 255.0 + 0.5);
    return (v.w << 24u) | (v.z << 16u) | (v.y << 8u) | v.x;
}

/// Pack 3x 4bit unorm + 1x 20bit
fn pack_unorm3x4_plus_unorm_20_(v: vec4<f32>) -> u32 {
    let sm = vec3<u32>(saturate(v.xyz) * 15.0 + 0.5);
    let bg = u32(saturate(v.w) * U20MAXF + 0.5);
    return (bg << 12u) | (sm.z << 8u) | (sm.y << 4u) | sm.x;
}

/// Unpack 3x 4bit unorm + 1x 20bit
fn unpack_unorm3x4_plus_unorm_20_(v: u32) -> vec4<f32> {
    return vec4(
        f32(v & 0xfu) / 15.0,
        f32((v >> 4u) & 0xFu) / 15.0,
        f32((v >> 8u) & 0xFu) / 15.0,
        f32((v >> 12u) & 0xFFFFFFu) / U20MAXF,
    );
}

// `vec3_to_rgb9e5_(vec3(1.0))`.
const RGB9E5_WHITE: u32 = 0x84020100u;

/// Packs a specular tint for the deferred specular tint texture.
///
/// The tint is stored as rgb9e5 XORed with the rgb9e5 encoding of white, so white packs to 0.
/// The texture clears to 0, and a `FragmentOutput` field that a shader does not write is 0, so
/// both decode to white.
fn pack_specular_tint(specular_tint: vec3<f32>) -> u32 {
    return vec3_to_rgb9e5_(specular_tint) ^ RGB9E5_WHITE;
}

/// Unpacks a specular tint packed by `pack_specular_tint`.
fn unpack_specular_tint(packed: u32) -> vec3<f32> {
    return rgb9e5_to_vec3_(packed ^ RGB9E5_WHITE);
}
