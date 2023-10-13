#import bevy_pbr::mesh_vertex_output MeshVertexOutput
struct WireframeMaterial {
    color: vec4<f32>,
};

@group(1) @binding(0)
var<uniform> material: WireframeMaterial;
@fragment
fn fragment(in: MeshVertexOutput) -> @location(0) vec4<f32> {
    return material.color;
}