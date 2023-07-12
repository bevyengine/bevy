#import bevy_pbr::mesh_functions as mesh_functions
#import bevy_render::view  View
#import bevy_pbr::mesh_types Mesh

@group(0) @binding(0)
var<uniform> view: View;

struct CustomMaterial {
    color: vec4<f32>,
};
@group(1) @binding(0)
var<uniform> material: CustomMaterial;

@group(2) @binding(0)
var<uniform> mesh: Mesh;

struct Vertex {
    @location(0) position: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
}

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;
    let world_position = mesh_functions::mesh_position_local_to_world(mesh.model, vec4(vertex.position, 1.0));
    out.position = mesh_functions::mesh_position_world_to_clip(world_position);
    return out;
}

@fragment
fn fragment() -> @location(0) vec4<f32> {
    return material.color;
}
