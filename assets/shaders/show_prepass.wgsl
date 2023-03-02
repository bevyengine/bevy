#import bevy_pbr::mesh_types
#import bevy_pbr::mesh_view_bindings
#import bevy_pbr::prepass_utils

struct ShowPrepassSettings {
    show_depth: u32,
    show_normals: u32,
    padding_1: u32,
    padding_2: u32,
}
@group(1) @binding(0)
var<uniform> settings: ShowPrepassSettings;

@fragment
fn fragment(
    @builtin(position) frag_coord: vec4<f32>,
    @builtin(sample_index) sample_index: u32,
    #import bevy_pbr::mesh_vertex_output
) -> @location(0) vec4<f32> {
    if settings.show_depth == 1u {
        let depth = prepass_depth(frag_coord, sample_index);
        return vec4(depth, depth, depth, 1.0);
    } else if settings.show_normals == 1u {
        let normal = prepass_normal(frag_coord, sample_index);
        return vec4(normal, 1.0);
    }

    return vec4(0.0);
}
