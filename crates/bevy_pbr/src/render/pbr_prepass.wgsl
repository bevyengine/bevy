#import bevy_pbr::prepass_bindings
#import bevy_pbr::pbr_bindings
#import bevy_pbr::pbr_functions

struct FragmentInput {
    @builtin(front_facing) is_front: bool,
    @builtin(position) frag_coord: vec4<f32>,
#ifdef VERTEX_UVS
    @location(0) uv: vec2<f32>,
#endif // VERTEX_UVS
#ifdef NORMAL_PREPASS
    @location(1) world_normal: vec3<f32>,
#ifdef VERTEX_TANGENTS
    @location(2) world_tangent: vec4<f32>,
#endif // VERTEX_TANGENTS
#endif // NORMAL_PREPASS
};

// We can use a simplified version of alpha_discard() here since we only need to handle the alpha_cutoff
fn prepass_alpha_discard(in: FragmentInput) {
#ifdef ALPHA_MASK
    var output_color: vec4<f32> = material.base_color;

#ifdef VERTEX_UVS
    if (material.flags & STANDARD_MATERIAL_FLAGS_BASE_COLOR_TEXTURE_BIT) != 0u {
        output_color = output_color * textureSample(base_color_texture, base_color_sampler, in.uv);
    }
#endif // VERTEX_UVS

    if ((material.flags & STANDARD_MATERIAL_FLAGS_ALPHA_MODE_MASK) != 0u) && output_color.a < material.alpha_cutoff {
        discard;
    }
#endif // ALPHA_MASK
}

#ifdef NORMAL_PREPASS

@fragment
fn fragment(in: FragmentInput) -> @location(0) vec4<f32> {
    prepass_alpha_discard(in);

    // NOTE: Unlit bit not set means == 0 is true, so the true case is if lit
    if (material.flags & STANDARD_MATERIAL_FLAGS_UNLIT_BIT) == 0u {
        let world_normal = prepare_world_normal(
            in.world_normal,
            (material.flags & STANDARD_MATERIAL_FLAGS_DOUBLE_SIDED_BIT) != 0u,
            in.is_front,
        );

        let normal = apply_normal_mapping(
            material.flags,
            world_normal,
#ifdef VERTEX_TANGENTS
#ifdef STANDARDMATERIAL_NORMAL_MAP
            in.world_tangent,
#endif // STANDARDMATERIAL_NORMAL_MAP
#endif // VERTEX_TANGENTS
#ifdef VERTEX_UVS
            in.uv,
#endif // VERTEX_UVS
        );

        return vec4(normal * 0.5 + vec3(0.5), 1.0);
    } else {
        return vec4(in.world_normal * 0.5 + vec3(0.5), 1.0);
    }
}

#else // NORMAL_PREPASS

@fragment
fn fragment(in: FragmentInput) {
    prepass_alpha_discard(in);
}

#endif // NORMAL_PREPASS

