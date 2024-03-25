#import bevy_pbr::mesh_functions as mesh_functions
#import bevy_pbr::{
    mesh_types::Mesh,
    view_transformations,
    forward_io::Vertex,
}
#import bevy_render::view::View

@group(0) @binding(0) var<uniform> view: View;
@group(1) @binding(0) var<uniform> mesh: Mesh;

struct CustomMaterial {
    color: vec4<f32>,
};
@group(2) @binding(0) var<uniform> material: CustomMaterial;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
}

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;
    let model = mesh_functions::get_model_matrix(vertex.instance_index);
    let world_position = mesh_functions::mesh_position_local_to_world(model, vec4<f32>(vertex.position, 1.0));
    out.position = view_transformations::position_world_to_clip(world_position.xyz);
    return out;
}

@fragment
fn fragment() -> @location(0) vec4<f32> {
    return material.color;
}
