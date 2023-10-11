#define_import_path bevy_pbr::meshlet_bindings

#import bevy_pbr::mesh_types

struct Vertex {
    position: vec3<f32>,
    normal: vec3<f32>,
    uv: vec2<f32>,
    tangent: vec4<f32>,
}

struct Meshlet {
    vertices_index: u32,
    indices_index: u32,
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

@group(1) @binding(0) var<storage, read> vertex_data: array<Vertex>;
@group(1) @binding(1) var<storage, read> meshlet_vertices: array<u32>;
@group(1) @binding(2) var<storage, read> meshlet_indices: array<u32>; // packed u8's
@group(1) @binding(3) var<storage, read> meshlets: array<Meshlet>;
@group(1) @binding(4) var<storage, read> instance_uniforms: array<Mesh>;
@group(1) @binding(5) var<storage, read> instanced_meshlet_instance_indices: array<u32>;
@group(1) @binding(6) var<storage, read> instanced_meshlet_meshlet_indices: array<u32>;
@group(1) @binding(7) var<storage, read> meshlet_bounding_spheres: array<MeshletBoundingSphere>;
@group(1) @binding(8) var<storage, write> draw_command_buffer: DrawIndexedIndirect;
@group(1) @binding(9) var<storage, write> draw_index_buffer: array<u32>;
