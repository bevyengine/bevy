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

var<private> flip_z: vec4<f32> = vec4<f32>(1.0, 1.0, -1.0, 1.0);

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
#ifdef SKINNED
    let model = skin_model(vertex.joint_indices, vertex.joint_weights);
#else
    let model = mesh.model;
#endif

    // NOTE: model is right-handed. Apply the right-handed transform to the right-handed vertex
    // position then flip the sign of the z component to make the result be left-handed y-up
    let world_position_lh = flip_z * mesh_position_local_to_world(
        model,
        vec4<f32>(vertex.position, 1.0)
    );
    var out: VertexOutput;
    // NOTE: The point light view_proj is left-handed
    out.clip_position = mesh_position_world_to_clip(world_position_lh);
    return out;
}
