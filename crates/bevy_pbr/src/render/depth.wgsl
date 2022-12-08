#define_import_path depth

#from bevy_pbr::mesh_view_types import View
#from bevy_pbr::mesh_bindings   import mesh
#from bevy_pbr::mesh_functions  import mesh_position_local_to_clip

@group(0) @binding(0)
var<uniform> view: ::View;

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
    let model = ::mesh.model;
#endif

    var out_depth_pipeline: VertexOutput;
    out_depth_pipeline.clip_position = ::mesh_position_local_to_clip(model, vec4<f32>(vertex.position, 1.0));
    return out_depth_pipeline;
}