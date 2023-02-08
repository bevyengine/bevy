#import bevy_pbr::mesh_types
#import bevy_pbr::mesh_view_bindings
#import bevy_pbr::prepass_utils

@group(1) @binding(0)
var<uniform> show_depth: f32;
@group(1) @binding(1)
var<uniform> show_normal: f32;

@fragment
fn fragment(
    @builtin(position) frag_coord: vec4<f32>,
    @builtin(sample_index) sample_index: u32,
    #import bevy_pbr::mesh_vertex_output
) -> @location(0) vec4<f32> {
    if show_depth == 1.0 {
        let depth = prepass_depth(frag_coord, sample_index);
        return vec4(depth, depth, depth, 1.0);
    } else if show_normal == 1.0 {
        let normal = prepass_normal(frag_coord, sample_index);
        return vec4(normal, 1.0);
    } else {
        // transparent
        return vec4(0.0);
    }
}
