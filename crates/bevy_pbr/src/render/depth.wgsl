#import bevy_pbr::mesh_struct

// NOTE: Keep in sync with pbr.wgsl
struct View {
    view_proj: mat4x4<f32>;
    projection: mat4x4<f32>;
    world_position: vec3<f32>;
};
[[group(0), binding(0)]]
var<uniform> view: View;

[[group(1), binding(0)]]
var<uniform> mesh: Mesh;

#ifdef SKINNED
[[group(2), binding(0)]]
var<uniform> joint_matrices: SkinnedMesh;
#import bevy_pbr::skinning
#endif

struct Vertex {
    [[location(0)]] position: vec3<f32>;
#ifdef SKINNED
    [[location(4)]] joint_indexes: vec4<u32>;
    [[location(5)]] joint_weights: vec4<f32>;
#endif
};

struct VertexOutput {
    [[builtin(position)]] clip_position: vec4<f32>;
};

[[stage(vertex)]]
fn vertex(vertex: Vertex) -> VertexOutput {
#ifdef SKINNED
    let model = skin_model(vertex.joint_indexes, vertex.joint_weights);
#else
    let model = mesh.model;
#endif

    var out: VertexOutput;
    out.clip_position = view.view_proj * model * vec4<f32>(vertex.position, 1.0);
    return out;
}
