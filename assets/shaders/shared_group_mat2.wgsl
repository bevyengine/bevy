#import bevy_pbr::mesh_view_bindings
#import bevy_pbr::mesh_bindings

#import bevy_pbr::mesh_functions
#import bevy_pbr::utils
#import bevy_pbr::clustered_forward
#import bevy_pbr::pbr_types
#import bevy_pbr::lighting
#import bevy_pbr::shadows
#import bevy_pbr::pbr_functions

#import example::shared_group::common

struct ReceiverMaterial {
    base_color: vec4<f32>,
};

@group(1) @binding(0)
var<uniform> material: ReceiverMaterial;


struct FragmentInput {
    @builtin(front_facing) is_front: bool,
    @builtin(position) frag_coord: vec4<f32>,
    #import bevy_pbr::mesh_vertex_output
};

@fragment
fn fragment(in: FragmentInput) -> @location(0) vec4<f32> {
    var pbr_input = pbr_input_new();

    // Set PBR material properties
    let impact = field_impact(in.world_position.xyz);
    pbr_input.material.base_color = material.base_color + vec4<f32>(impact, impact, impact, 1.0);

    // Set PBR frament / world properties
    pbr_input.frag_coord = in.frag_coord;
    pbr_input.world_position = in.world_position;
    pbr_input.world_normal = in.world_normal;

    // Calculate stuff?
    pbr_input.N = prepare_normal(pbr_input.material.flags, in.world_normal, in.uv, in.is_front);
    pbr_input.V = calculate_view(in.world_position, pbr_input.is_orthographic);

    return pbr(pbr_input);
}