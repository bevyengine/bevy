#import bevy_pbr::mesh_types
#import bevy_pbr::mesh_view_bindings
#import bevy_pbr::prepass_utils
#from bevy_pbr::mesh_vertex_output import MeshVertexOutput

@group(1) @binding(0)
var<uniform> show_depth: f32;
@group(1) @binding(1)
var<uniform> show_normal: f32;

@fragment
fn fragment(
    @builtin(sample_index) sample_index: u32,
    mesh: ::MeshVertexOutput,
) -> @location(0) vec4<f32> {
    if show_depth == 1.0 {
        let depth = bevy_pbr::prepass_utils::prepass_depth(mesh.clip_position, sample_index);
        return vec4(depth, depth, depth, 1.0);
    } else if show_normal == 1.0 {
        let normal = bevy_pbr::prepass_utils::prepass_normal(mesh.clip_position, sample_index);
        return vec4(normal, 1.0);
    } else {
        // transparent
        return vec4(0.0);
    }
}
