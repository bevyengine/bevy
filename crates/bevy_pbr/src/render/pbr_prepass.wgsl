#import bevy_pbr::pbr_prepass_functions
#import bevy_pbr::pbr_bindings
#import bevy_pbr::pbr_types
#ifdef NORMAL_PREPASS
#import bevy_pbr::pbr_functions
#endif // NORMAL_PREPASS

#import bevy_pbr::prepass_io as prepass_io
#import bevy_pbr::mesh_view_bindings view
 
#ifdef PREPASS_FRAGMENT
@fragment
fn fragment(
    in: prepass_io::FragmentInput,
    @builtin(front_facing) is_front: bool,
) -> prepass_io::FragmentOutput {
    bevy_pbr::pbr_prepass_functions::prepass_alpha_discard(in);

    var out: prepass_io::FragmentOutput;

#ifdef DEPTH_CLAMP_ORTHO
    out.frag_depth = in.clip_position_unclamped.z;
#endif // DEPTH_CLAMP_ORTHO

#ifdef NORMAL_PREPASS
    // NOTE: Unlit bit not set means == 0 is true, so the true case is if lit
    if (bevy_pbr::pbr_bindings::material.flags & bevy_pbr::pbr_types::STANDARD_MATERIAL_FLAGS_UNLIT_BIT) == 0u {
        let world_normal = bevy_pbr::pbr_functions::prepare_world_normal(
            in.world_normal,
            (bevy_pbr::pbr_bindings::material.flags & bevy_pbr::pbr_types::STANDARD_MATERIAL_FLAGS_DOUBLE_SIDED_BIT) != 0u,
            is_front,
        );

        let normal = bevy_pbr::pbr_functions::apply_normal_mapping(
            bevy_pbr::pbr_bindings::material.flags,
            world_normal,
#ifdef VERTEX_TANGENTS
#ifdef STANDARDMATERIAL_NORMAL_MAP
            in.world_tangent,
#endif // STANDARDMATERIAL_NORMAL_MAP
#endif // VERTEX_TANGENTS
#ifdef VERTEX_UVS
            in.uv,
#endif // VERTEX_UVS
            view.mip_bias,
        );

        out.normal = vec4(normal * 0.5 + vec3(0.5), 1.0);
    } else {
        out.normal = vec4(in.world_normal * 0.5 + vec3(0.5), 1.0);
    }
#endif // NORMAL_PREPASS

#ifdef MOTION_VECTOR_PREPASS
    out.motion_vector = bevy_pbr::pbr_prepass_functions::calculate_motion_vector(in.world_position, in.previous_world_position);
#endif

    return out;
}
#else
@fragment
fn fragment(in: prepass_io::FragmentInput) {
    bevy_pbr::pbr_prepass_functions::prepass_alpha_discard(in);
}
#endif // PREPASS_FRAGMENT
