#import bevy_pbr::meshlet_bindings meshlet_thread_meshlet_ids, meshlets, meshlet_vertex_ids, meshlet_vertex_data, meshlet_thread_instance_ids, meshlet_instance_uniforms

struct VertexOutput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) tangent: vec4<f32>,
}

@vertex
fn vertex(@builtin(vertex_index) packed_meshlet_index: u32) -> VertexOutput {
    let thead_id = packed_meshlet_index >> 8u;
    let meshlet_id = meshlet_thread_meshlet_ids[thead_id];
    let meshlet = meshlets[meshlet_id];
    let index = extractBits(packed_meshlet_index, 0u, 8u);
    let vertex_id = meshlet_vertex_ids[meshlet.start_vertex_id + index];
    let vertex = meshlet_vertex_data[vertex_id];
    let instance_id = meshlet_thread_instance_ids[thead_id];
    let instance_uniform = meshlet_instance_uniforms[instance_id];

    // TODO
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    // TODO
}
