#import bevy_pbr::mesh_bindings    mesh
#import bevy_pbr::mesh_functions   mesh_position_local_to_clip

#ifdef SKINNED
    #import bevy_pbr::skinning
#endif

struct Vertex {
    @location(0) position: vec3<f32>,
#ifdef SKINNED
    @location(4) joint_indexes: vec4<u32>,
    @location(5) joint_weights: vec4<f32>,
#endif
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
};

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
#ifdef SKINNED
    let model = bevy_pbr::skinning::skin_model(vertex.joint_indexes, vertex.joint_weights);
#else
    let model = mesh.model;
#endif

    var out: VertexOutput;
    out.clip_position = mesh_position_local_to_clip(model, vec4<f32>(vertex.position, 1.0));
    return out;
}

@fragment
fn fragment() -> @location(0) vec4<f32> {
    return vec4<f32>(1.0, 1.0, 1.0, 1.0);
}
