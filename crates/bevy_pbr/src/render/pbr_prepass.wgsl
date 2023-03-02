#import bevy_pbr::prepass_bindings
#import bevy_pbr::pbr_bindings
#ifdef NORMAL_PREPASS
#import bevy_pbr::pbr_functions
#endif // NORMAL_PREPASS

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

// Cutoff used for the premultiplied alpha modes BLEND and ADD.
const PREMULTIPLIED_ALPHA_CUTOFF = 0.05;

// We can use a simplified version of alpha_discard() here since we only need to handle the alpha_cutoff
fn prepass_alpha_discard(in: FragmentInput) {

// This is a workaround since the preprocessor does not support
// #if defined(ALPHA_MASK) || defined(BLEND_PREMULTIPLIED_ALPHA)
#ifndef ALPHA_MASK
#ifndef BLEND_PREMULTIPLIED_ALPHA

#define EMPTY_PREPASS_ALPHA_DISCARD

#endif // BLEND_PREMULTIPLIED_ALPHA not defined
#endif // ALPHA_MASK not defined

#ifndef EMPTY_PREPASS_ALPHA_DISCARD
    var output_color: vec4<f32> = material.base_color;

#ifdef VERTEX_UVS
    if (material.flags & STANDARD_MATERIAL_FLAGS_BASE_COLOR_TEXTURE_BIT) != 0u {
        output_color = output_color * textureSample(base_color_texture, base_color_sampler, in.uv);
    }
#endif // VERTEX_UVS

#ifdef ALPHA_MASK
    if ((material.flags & STANDARD_MATERIAL_FLAGS_ALPHA_MODE_MASK) != 0u) && output_color.a < material.alpha_cutoff {
        discard;
    }
#endif // ALPHA_MASK

#ifdef BLEND_PREMULTIPLIED_ALPHA
    let alpha_mode = material.flags & STANDARD_MATERIAL_FLAGS_ALPHA_MODE_RESERVED_BITS;
    if (alpha_mode == STANDARD_MATERIAL_FLAGS_ALPHA_MODE_BLEND || alpha_mode == STANDARD_MATERIAL_FLAGS_ALPHA_MODE_ADD) 
        && output_color.a < PREMULTIPLIED_ALPHA_CUTOFF {
        discard;
    } else if alpha_mode == STANDARD_MATERIAL_FLAGS_ALPHA_MODE_PREMULTIPLIED 
        && all(output_color < vec4(PREMULTIPLIED_ALPHA_CUTOFF)) {
        discard;
    }
#endif // BLEND_PREMULTIPLIED_ALPHA

#endif // EMPTY_PREPASS_ALPHA_DISCARD not defined
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

