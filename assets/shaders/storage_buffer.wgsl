#import bevy_pbr::{
    mesh_functions,
    view_transformations::position_world_to_clip
}

@group(2) @binding(0) var<storage, read> colors: array<vec4<f32>, 5>;

struct Vertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) color: vec4<f32>,
};

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;
    var world_from_local = mesh_functions::get_world_from_local(vertex.instance_index);
    out.world_position = mesh_functions::mesh_position_local_to_world(world_from_local, vec4(vertex.position, 1.0));
    out.clip_position = position_world_to_clip(out.world_position.xyz);

    // We have 5 colors in the storage buffer, but potentially many instances of the mesh, so
    // we use the instance index to select a color from the storage buffer.
    out.color = colors[vertex.instance_index % 5];

    return out;
}

@fragment
fn fragment(
    mesh: VertexOutput,
) -> @location(0) vec4<f32> {
    return mesh.color;
}