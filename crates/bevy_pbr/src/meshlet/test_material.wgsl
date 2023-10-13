#import bevy_pbr::meshlet_bindings vertex_data, meshlet_vertices, meshlets, instance_uniforms, instanced_meshlet_instance_indices, instanced_meshlet_meshlet_indices

struct VertexOutput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) tangent: vec4<f32>,
}

@vertex
fn vertex(@builtin(vertex_index) packed_meshlet_index: u32) -> VertexOutput {
    let instanced_meshlet_index = packed_meshlet_index >> 8u;
    let meshlet_index = instanced_meshlet_meshlet_indices[instanced_meshlet_index];
    let meshlet = meshlets[meshlet_index];
    let meshlet_vertex_index = extractBits(packed_meshlet_index, 0u, 8u);
    let meshlet_vertex = meshlet_vertices[meshlet.vertices_index + meshlet_vertex_index];
    let vertex = vertex_data[meshlet_vertex];
    let mesh_uniform = instance_uniforms[instanced_meshlet_index];

    // TODO
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    // TODO
}
