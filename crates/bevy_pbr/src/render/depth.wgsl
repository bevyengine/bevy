#import bevy_pbr::mesh_struct

// NOTE: Keep in sync with pbr.wgsl
struct View {
    view_proj: mat4x4<f32>;
    projection: mat4x4<f32>;
    world_position: vec3<f32>;
};
[[group(0), binding(0)]]
var<uniform> view: View;

struct Vertex {
    [[location(0)]] i_model_col0: vec3<f32>;
    [[location(1)]] i_model_col1: vec3<f32>;
    [[location(2)]] i_model_col2: vec3<f32>;
    [[location(3)]] i_model_col3: vec3<f32>;
    [[location(4)]] i_inverse_model_col0: vec3<f32>;
    [[location(5)]] i_inverse_model_col1: vec3<f32>;
    [[location(6)]] i_inverse_model_col2: vec3<f32>;
    [[location(7), interpolate(flat)]] i_mesh_flags: u32;
    [[location(8)]] position: vec3<f32>;
};

struct VertexOutput {
    [[builtin(position)]] clip_position: vec4<f32>;
};

fn vec3_to_mat4x4(c0: vec3<f32>, c1: vec3<f32>, c2: vec3<f32>, c3: vec3<f32>) -> mat4x4<f32> {
    // NOTE: The shader compiler will optimize away multiplications by 0.0 and 1.0 and not use
    // registers for unused parts of the matrix
    return mat4x4<f32>(
        vec4<f32>(c0, 0.0),
        vec4<f32>(c1, 0.0),
        vec4<f32>(c2, 0.0),
        vec4<f32>(c3, 1.0)
    );
}

[[stage(vertex)]]
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;
    let model = vec3_to_mat4x4(
        vertex.i_model_col0,
        vertex.i_model_col1,
        vertex.i_model_col2,
        vertex.i_model_col3
    );
    out.clip_position = view.view_proj * model * vec4<f32>(vertex.position, 1.0);
    return out;
}
