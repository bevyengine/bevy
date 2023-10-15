#define_import_path bevy_pbr::meshlet_bindings

#import bevy_pbr::mesh_types Mesh
#import bevy_render::view View

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

struct Meshlet {
    start_vertex_id: u32,
    start_index_id: u32,
    vertex_count: u32,
    triangle_count: u32,
}

struct MeshletBoundingSphere {
    center: vec3<f32>,
    radius: f32,
}

struct DrawIndexedIndirect {
    vertex_count: atomic<u32>,
    instance_count: u32,
    base_index: u32,
    vertex_offset: u32,
    base_instance: u32,
}

#ifndef MESHLET_CULLING_BINDINGS
@group(#{MESHLET_BIND_GROUP}) @binding(0) var<storage, read> meshlet_vertex_data: array<PackedVertex>;
@group(#{MESHLET_BIND_GROUP}) @binding(1) var<storage, read> meshlet_vertex_ids: array<u32>;
#endif
@group(#{MESHLET_BIND_GROUP}) @binding(2) var<storage, read> meshlets: array<Meshlet>;
@group(#{MESHLET_BIND_GROUP}) @binding(3) var<storage, read> meshlet_instance_uniforms: array<Mesh>;
@group(#{MESHLET_BIND_GROUP}) @binding(4) var<storage, read> meshlet_thread_instance_ids: array<u32>;
@group(#{MESHLET_BIND_GROUP}) @binding(5) var<storage, read> meshlet_thread_meshlet_ids: array<u32>;
#ifdef MESHLET_CULLING_BINDINGS
@group(#{MESHLET_BIND_GROUP}) @binding(6) var<storage, read> meshlet_indices: array<u32>; // packed u8's
@group(#{MESHLET_BIND_GROUP}) @binding(7) var<storage, read> meshlet_bounding_spheres: array<MeshletBoundingSphere>;
@group(#{MESHLET_BIND_GROUP}) @binding(8) var<storage, read_write> draw_command_buffer: DrawIndexedIndirect;
@group(#{MESHLET_BIND_GROUP}) @binding(9) var<storage, write> draw_index_buffer: array<u32>;
@group(#{MESHLET_BIND_GROUP}) @binding(10) var<uniform> view: View;

fn get_meshlet_index(index_id: u32) -> u32 {
    let packed_index = meshlet_indices[index_id / 4u];
    let bit_offset = (index_id % 4u) * 8u;
    return extractBits(packed_index, bit_offset, 8u);
}
#endif
