#import bevy_pbr::mesh_view_types
#import bevy_pbr::mesh_types

@group(0) @binding(0)
var<uniform> view: View;

@group(1) @binding(0)
var<uniform> mesh: Mesh;

#ifdef SKINNED
@group(1) @binding(1)
var<uniform> joint_matrices: SkinnedMesh;
#import bevy_pbr::skinning
#endif

// NOTE: Bindings must come before functions that use them!
#import bevy_pbr::mesh_functions

struct Vertex {
    @location(0) position: vec3<f32>,
#ifdef SKINNED
    @location(4) joint_indices: vec4<u32>,
    @location(5) joint_weights: vec4<f32>,
#endif
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
};

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
#ifdef SKINNED
    let model = skin_model(vertex.joint_indices, vertex.joint_weights);
#else
    let model = mesh.model;
#endif

    var out: VertexOutput;
    out.clip_position = mesh_position_local_to_clip(model, vec4<f32>(vertex.position, 1.0));
#ifdef DEPTH_CLAMP_ORTHO
        out.clip_position.z = min(out.clip_position.z, 1.0);
#endif

    return out;
}
