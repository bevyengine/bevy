#import bevy_pbr::mesh_types
#import bevy_pbr::mesh_view_bindings

[[group(1), binding(0)]]
var<uniform> mesh: Mesh;

// NOTE: Bindings must come before functions that use them!
#import bevy_pbr::mesh_functions

struct Vertex {
    [[location(0)]] position: vec3<f32>;
    [[location(1)]] normal: vec3<f32>;
    [[location(2)]] uv: vec2<f32>;
};

struct VertexOutput {
    [[builtin(position)]] clip_position: vec4<f32>;
};

[[stage(vertex)]]
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = mesh_position_local_to_clip(mesh.model, vec4<f32>(vertex.position, 1.0));
    return out;
}

[[stage(fragment)]]
fn fragment() -> [[location(0)]] vec4<f32> {
    var color = vec4<f32>(0.0, 0.0, 1.0, 1.0);
# ifdef IS_RED
    color = vec4<f32>(1.0, 0.0, 0.0, 1.0);
# endif
    return color;
}
