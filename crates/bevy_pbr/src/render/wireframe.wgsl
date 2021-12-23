#import bevy_pbr::mesh_view_types
#import bevy_pbr::mesh_view_bindings
#import bevy_pbr::mesh_types

[[group(1), binding(0)]]
var<uniform> mesh: Mesh;

// NOTE: Bindings must come before functions that use them!
#import bevy_pbr::mesh_functions

struct Vertex {
    [[location(0)]] position: vec3<f32>;
};

struct VertexOutput {
    [[builtin(position)]] clip_position: vec4<f32>;
};

[[stage(vertex)]]
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = mesh_model_position_to_clip(vec4<f32>(vertex.position, 1.0));
    return out;
}

[[stage(fragment)]]
fn fragment() -> [[location(0)]] vec4<f32> {
    return vec4<f32>(1.0, 1.0, 1.0, 1.0);
}
