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

struct EmitterMaterial {
    base_color: vec4<f32>,
};

@group(1) @binding(0)
var<uniform> material: EmitterMaterial;


struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    #import bevy_pbr::mesh_vertex_output
};

@vertex
fn vertex(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    let scale = 1.0 + field_phase(0.0) * 0.01;
    let scaled_position = vec4<f32>(in.position * vec3<f32>(scale, scale, scale), 1.0);
    
    out.world_normal = mesh_normal_local_to_world(in.normal);
    out.world_position = mesh_position_local_to_world(mesh.model, scaled_position);
    out.uv = in.uv;
    out.clip_position = mesh_position_world_to_clip(out.world_position);

    return out;
}

struct FragmentInput {
    @builtin(front_facing) is_front: bool,
    @builtin(position) frag_coord: vec4<f32>,
    #import bevy_pbr::mesh_vertex_output
};

@fragment
fn fragment(in: FragmentInput) -> @location(0) vec4<f32> {
    var pbr_input = pbr_input_new();

    // Set PBR material properties
    pbr_input.material.base_color = material.base_color;

    // Set PBR frament / world properties
    pbr_input.frag_coord = in.frag_coord;
    pbr_input.world_position = in.world_position;
    pbr_input.world_normal = in.world_normal;
    pbr_input.N = prepare_normal(pbr_input.material.flags, in.world_normal, in.uv, in.is_front);
    pbr_input.V = calculate_view(in.world_position, pbr_input.is_orthographic);

    return pbr(pbr_input);
}