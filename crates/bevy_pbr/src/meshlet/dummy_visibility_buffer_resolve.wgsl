/// Dummy shader to prevent naga_oil from complaining about missing imports when the MeshletPlugin is not loaded,
/// as naga_oil tries to resolve imports even if they're behind an #ifdef.

#define_import_path bevy_pbr::meshlet_visibility_buffer_resolve

struct VertexOutput {
    dummy: u32
}

fn resolve_vertex_output(frag_coord: vec4<f32>) -> VertexOutput {
    return VertexOutput(1717u);
}
