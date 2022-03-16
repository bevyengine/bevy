#import bevy_pbr::mesh_struct

// NOTE: Keep in sync with pbr.wgsl
struct View {
    view_proj: mat4x4<f32>;
    view: mat4x4<f32>;
    inverse_view: mat4x4<f32>;
    projection: mat4x4<f32>;
    world_position: vec3<f32>;
    near: f32;
    far: f32;
    width: f32;
    height: f32;
};
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

var<private> flip_z: vec4<f32> = vec4<f32>(1.0, 1.0, -1.0, 1.0);

[[stage(vertex)]]
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;
    // NOTE: mesh.model is right-handed. Apply the right-handed transform to the right-handed vertex position
    //       then flip the sign of the z component to make the result be left-handed y-up
    // NOTE: The point light view_proj is left-handed
    out.clip_position = view.view_proj * ((mesh.model * vec4<f32>(vertex.position, 1.0)) * flip_z);
    return out;
}
