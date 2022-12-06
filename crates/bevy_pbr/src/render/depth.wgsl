#define_import_path depth

#import bevy_render::core_types
#import bevy_pbr::mesh_bindings
#import bevy_pbr::mesh_functions

#ifdef SKINNED
#import bevy_pbr::skinning
#endif

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
    let model = bevy_pbr::skinning::skin_model(vertex.joint_indices, vertex.joint_weights);
#else
    let model = bevy_pbr::mesh_bindings::mesh.model;
#endif

    var out_depth_pipeline: VertexOutput;
    out_depth_pipeline.clip_position = bevy_pbr::mesh_functions::mesh_position_local_to_clip(model, vec4<f32>(vertex.position, 1.0));
    return out_depth_pipeline;
}