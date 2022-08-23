#import bevy_pbr::mesh_vertex_output

struct LineMaterial {
    color: vec4<f32>,
};

@group(1) @binding(0)
var<uniform> material: LineMaterial;

@fragment
fn fragment(
    mesh: bevy_pbr::mesh_vertex_output::MeshVertexOutput,
) -> @location(0) vec4<f32> {
    return material.color;
}
