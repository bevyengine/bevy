#define_import_path bevy_solari::scene_bindings

struct PackedVertex {
    a: vec4<f32>,
    b: vec4<f32>,
    tangent: vec4<f32>,
}

struct Vertex {
    position: vec3<f32>,
    normal: vec3<f32>,
    uv: vec2<f32>,
    tangent: vec4<f32>,
}

fn unpack_vertex(packed: PackedVertex) -> Vertex {
    var vertex: Vertex;
    vertex.position = packed.a.xyz;
    vertex.normal = vec3(packed.a.w, packed.b.xy);
    vertex.uv = packed.b.zw;
    vertex.tangent = packed.tangent;
    return vertex;
}

@group(0) @binding(0) var<storage> vertex_buffers: binding_array<array<PackedVertex>>;
@group(0) @binding(1) var<storage> index_buffers: binding_array<array<u32>>;
// @group(0) @binding(2) var textures: binding_array<texture_2d<f32>>;
// @group(0) @binding(3) var samplers: binding_array<sampler>;
@group(0) @binding(4) var tlas: acceleration_structure;
@group(0) @binding(5) var<storage> transforms: array<mat4x4<f32>>;
// @group(0) @binding(6) var<storage> mesh_material_ids: array<u32>;
// @group(0) @binding(7) var<storage> materials: array<Material>;
// @group(0) @binding(8) var<storage> light_sources: array<LightSource>;
// @group(0) @binding(9) var<storage> directional_lights: array<DirectionalLight>;
