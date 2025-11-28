#import bevy_pbr::{
    mesh_functions::{get_world_from_local, mesh_position_local_to_clip}
}

@group(#{MATERIAL_BIND_GROUP}) @binding(100) var<uniform> outline_color: vec4<f32>;

const OUTLINE_WIDTH = 0.1;

struct Vertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
};

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;

    // This only works when the mesh is at the origin.
    let expanded_position = vertex.position * (1 + OUTLINE_WIDTH);

    out.clip_position = mesh_position_local_to_clip(
        get_world_from_local(vertex.instance_index),
        vec4<f32>(expanded_position, 1.0),
    );
    return out;
}

@fragment
fn fragment(input: VertexOutput) -> @location(0) vec4<f32> {
    return outline_color;
}