#define_import_path bevy_pbr::pbr_deferred_types
#import bevy_pbr::mesh_types MESH_FLAGS_SHADOW_RECEIVER_BIT 
#import bevy_pbr::pbr_types STANDARD_MATERIAL_FLAGS_FOG_ENABLED_BIT, STANDARD_MATERIAL_FLAGS_UNLIT_BIT

// Maximum of 8 bits available
const DEFERRED_FLAGS_UNLIT_BIT: u32                 = 1u;
const DEFERRED_FLAGS_FOG_ENABLED_BIT: u32           = 2u;
const DEFERRED_MESH_FLAGS_SHADOW_RECEIVER_BIT: u32  = 4u;

fn deferred_flags_from_mesh_mat_flags(mesh_flags: u32, mat_flags: u32) -> u32 {
    var flags = 0u;
    flags |= u32((mesh_flags & MESH_FLAGS_SHADOW_RECEIVER_BIT) != 0u) * DEFERRED_MESH_FLAGS_SHADOW_RECEIVER_BIT;
    flags |= u32((mat_flags & STANDARD_MATERIAL_FLAGS_FOG_ENABLED_BIT) != 0u) * DEFERRED_FLAGS_FOG_ENABLED_BIT;
    flags |= u32((mat_flags & STANDARD_MATERIAL_FLAGS_UNLIT_BIT) != 0u) * DEFERRED_FLAGS_UNLIT_BIT;
    return flags;
}

fn mesh_mat_flags_from_deferred_flags(deferred_flags: u32) -> vec2<u32> {
    var mat_flags = 0u;
    var mesh_flags = 0u;
    mesh_flags |= u32((deferred_flags & DEFERRED_MESH_FLAGS_SHADOW_RECEIVER_BIT) != 0u) * MESH_FLAGS_SHADOW_RECEIVER_BIT;
    mat_flags |= u32((deferred_flags & DEFERRED_FLAGS_FOG_ENABLED_BIT) != 0u) * STANDARD_MATERIAL_FLAGS_FOG_ENABLED_BIT;
    mat_flags |= u32((deferred_flags & DEFERRED_FLAGS_UNLIT_BIT) != 0u) * STANDARD_MATERIAL_FLAGS_UNLIT_BIT;
    return vec2(mesh_flags, mat_flags);
}


// For stroing normals as oct24
// Flags are stored in the remaining 8 bits
// https://jcgt.org/published/0003/02/01/paper.pdf
// Could possibly go down to oct20 if the space is needed 

fn octa_wrap(v: vec2<f32>) -> vec2<f32> {
    return (1.0 - abs(v.yx)) * select(vec2(-1.0), vec2(1.0), v.xy >= vec2(0.0));
}

fn octa_encode(n: vec3<f32>) -> vec2<f32> {
    var n = n / (abs(n.x) + abs(n.y) + abs(n.z));
    if (n.z < 0.0) {
        n = vec3(octa_wrap(n.xy), n.z);
    }
    return n.xy * 0.5 + 0.5;
}

fn octa_decode(f: vec2<f32>) -> vec3<f32> {
    var f = f * 2.0 - 1.0;
    var n = vec3( f.x, f.y, 1.0 - abs(f.x) - abs(f.y));
    if (n.z < 0.0) {
        n = vec3(octa_wrap(n.xy), n.z);
    }
    return normalize(n);
}

const U12MAXF = 4095.0;
const U16MAXF = 65535.0;
const U20MAXF = 1048575.0;

fn pack_24bit_nor_and_flags(oct_nor: vec2<f32>, flags: u32) -> u32 {
    let unorm1 = u32(saturate(oct_nor.x) * U12MAXF + 0.5);
    let unorm2 = u32(saturate(oct_nor.y) * U12MAXF + 0.5);
    return (unorm1 & 0xFFFu) | ((unorm2 & 0xFFFu) << 12u) | ((flags & 0xFFu) << 24u);
}

fn unpack_24bit_nor(packed: u32) -> vec2<f32> {
    let unorm1 = packed & 0xFFFu;
    let unorm2 = (packed >> 12u) & 0xFFFu;
    return vec2(f32(unorm1) / U12MAXF, f32(unorm2) / U12MAXF);
}

fn unpack_flags(packed: u32) -> u32 {
    return (packed >> 24u) & 0xFFu;
}

// The builtin one didn't work in webgl
// "'unpackUnorm4x8' : no matching overloaded function found"
fn unpack_unorm4x8_(v: u32) -> vec4<f32> {
    return vec4(
        f32(v & 0xffu),
        f32((v >> 8u) & 0xffu),
        f32((v >> 16u) & 0xffu),
        f32((v >> 24u) & 0xffu)
    ) / 255.0;
}

// 'packUnorm4x8' : no matching overloaded function found
fn pack_unorm4x8_(v: vec4<f32>) -> u32 {
    let v = vec4<u32>(saturate(v) * 255.0 + 0.5);
    return (v.w << 24u) | (v.z << 16u) | (v.y << 8u) | v.x;
}

// pack 3x 4bit unorm + 1x 20bit
fn pack_unorm3x4_plus_unorm_20_(v: vec4<f32>) -> u32 {
    let sm = vec3<u32>(saturate(v.xyz) * 15.0 + 0.5);
    let bg = u32(saturate(v.w) * U20MAXF + 0.5);
    return (bg << 12u) | (sm.z << 8u) | (sm.y << 4u) | sm.x;
}

// unpack 3x 4bit unorm + 1x 20bit
fn unpack_unorm3x4_plus_unorm_20_(v: u32) -> vec4<f32> {
    return vec4(
        f32(v & 0xfu) / 15.0,
        f32((v >> 4u) & 0xFu) / 15.0,
        f32((v >> 8u) & 0xFu) / 15.0,
        f32((v >> 12u) & 0xFFFFFFu) / U20MAXF,
    );
}
