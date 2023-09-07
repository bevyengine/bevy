#define_import_path bevy_solari::scene_types

struct SolariUniforms {
    frame_count: u32,
    sun_direction: vec3<f32>,
    sun_color: vec3<f32>,
}

struct SolariIndexBuffer {
    buffer: array<u32>,
}

struct SolariVertexBuffer {
    buffer: array<SolariPackedVertex>,
}

// The size of a vertex is 48 bytes of data
//
// The size of the SolariVertex struct when used in an
// array is padded to 64 bytes due to WGSL alignment rules
//
// This struct is properly 48 bytes
struct SolariPackedVertex {
    a: vec4<f32>,
    b: vec4<f32>,
    local_tangent: vec4<f32>,
}

fn unpack_vertex(packed: SolariPackedVertex) -> SolariVertex {
    var vertex: SolariVertex;
    vertex.local_position = packed.a.xyz;
    vertex.local_normal = vec3(packed.a.w, packed.b.xy);
    vertex.local_tangent = packed.local_tangent;
    vertex.uv = packed.b.zw;
    return vertex;
}

struct SolariVertex {
    local_position: vec3<f32>,
    local_normal: vec3<f32>,
    local_tangent: vec4<f32>,
    uv: vec2<f32>,
}

const TEXTURE_MAP_NONE = 0xffffffffu;

struct SolariMaterial {
    base_color: vec4<f32>,
    base_color_texture_index: u32,
    normal_map_texture_index: u32,
    emissive: vec3<f32>,
    emissive_texture_index: u32,
}

struct SolariSampledMaterial {
    base_color: vec3<f32>,
    emissive: vec3<f32>,
}

struct SolariRayHit {
    world_position: vec3<f32>,
    world_normal: vec3<f32>,
    geometric_world_normal: vec3<f32>,
    uv: vec2<f32>,
    material: SolariSampledMaterial,
}
