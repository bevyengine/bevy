#import bevy_pbr::mesh_view_bind_group
#import bevy_pbr::mesh_struct

struct Vertex {
    [[location(0)]] position: vec3<f32>;
    [[location(1)]] blend_color: vec4<f32>;
};

struct CustomMaterial {
    color: vec4<f32>;
};
[[group(1), binding(0)]]
var<uniform> material: CustomMaterial;

[[group(2), binding(0)]]
var<uniform> mesh: Mesh;

struct VertexOutput {
    [[builtin(position)]] clip_position: vec4<f32>;
    [[location(0)]] blend_color: vec4<f32>;
};

[[stage(vertex)]]
fn vertex(vertex: Vertex) -> VertexOutput {
    let world_position = mesh.model * vec4<f32>(vertex.position, 1.0);

    var out: VertexOutput;
    out.clip_position = view.view_proj * world_position;
    out.blend_color = vertex.blend_color;
    return out;
}

struct FragmentInput {
    [[location(0)]] blend_color: vec4<f32>;
};

[[stage(fragment)]]
fn fragment(input: FragmentInput) -> [[location(0)]] vec4<f32> {
    return material.color * input.blend_color;
}
