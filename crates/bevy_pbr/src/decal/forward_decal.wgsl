#define_import_path bevy_pbr::decal::forward

#import bevy_pbr::{
    forward_io::VertexOutput,
    mesh_functions::get_world_from_local,
    mesh_view_bindings::view,
    pbr_functions::calculate_tbn_mikktspace,
    prepass_utils::prepass_depth,
    view_transformations::depth_ndc_to_view_z,
}
#import bevy_render::maths::project_onto

@group(#{MATERIAL_BIND_GROUP}) @binding(200)
var<uniform> inv_depth_fade_factor: f32;

struct ForwardDecalInformation {
    world_position: vec4<f32>,
    uv: vec2<f32>,
    alpha: f32,
}

fn get_forward_decal_info(in: VertexOutput) -> ForwardDecalInformation {
    let world_from_local = get_world_from_local(in.instance_index);
    let scale = (world_from_local * vec4(1.0, 1.0, 1.0, 0.0)).xyz;
    let scaled_tangent = vec4(in.world_tangent.xyz / scale, in.world_tangent.w);

    let V = normalize(view.world_position - in.world_position.xyz);

    // Transform V from fragment to camera in world space to tangent space.
    let TBN = calculate_tbn_mikktspace(in.world_normal, scaled_tangent);
    let T = TBN[0];
    let B = TBN[1];
    let N = TBN[2];
    let Vt = vec3(dot(V, T), dot(V, B), dot(V, N));

    let frag_depth = depth_ndc_to_view_z(in.position.z);
    let depth_pass_depth = depth_ndc_to_view_z(prepass_depth(in.position, 0u));
    let diff_depth = frag_depth - depth_pass_depth;
    let diff_depth_abs = abs(diff_depth);

    // Apply UV parallax
    let contact_on_decal = project_onto(V * diff_depth, in.world_normal);
    let normal_depth = length(contact_on_decal);
    let view_steepness = abs(Vt.z);
    let delta_uv = normal_depth * Vt.xy * vec2(1.0, -1.0) / view_steepness;
    let uv = in.uv + delta_uv;

    let world_position = vec4(in.world_position.xyz + V * diff_depth_abs, in.world_position.w);
    let alpha = saturate(1.0 - (normal_depth * inv_depth_fade_factor));

    return ForwardDecalInformation(world_position, uv, alpha);
}
