#import bevy_pbr::meshlet_bindings vertex_data, meshlet_vertices, meshlets, instance_uniforms, instanced_meshlet_instance_indices, instanced_meshlet_meshlet_indices

struct VertexOutput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) tangent: vec4<f32>,
}

@vertex
fn vertex(@builtin(vertex_index) packed_meshlet_index: u32) -> VertexOutput {
    // TODO
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    // TODO
}
