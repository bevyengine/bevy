#import bevy_pbr::mesh_view_types
#import bevy_pbr::mesh_types
#import bevy_pbr::mesh_functions

[[group(0), binding(0)]]
var<uniform> view: View;

[[group(1), binding(0)]]
var<uniform> mesh: Mesh;

struct Vertex {
    [[location(0)]] position: vec3<f32>;
};

struct VertexOutput {
    [[builtin(position)]] clip_position: vec4<f32>;
};

[[stage(vertex)]]
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = mesh_model_position_to_clip(
        mesh.model,
        view.view_proj,
        vec4<f32>(vertex.position, 1.0)
    );
    return out;
}
