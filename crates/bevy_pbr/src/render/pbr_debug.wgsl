#define_import_path bevy_pbr::pbr_debug

#ifdef PBR_DEBUG

// Used when debug stuff is missing
var<private> pink: vec4<f32> = vec4<f32>(1.0, 0.0, 1.0, 1.0);

fn pbr_debug(in: FragmentInput) -> vec4<f32> {
#ifdef PBR_DEBUG_UVS

#ifdef VERTEX_UVS
    return vec4<f32>(in.uv, 0.0, 1.0);
#else
    return pink;
#endif // VERTEX_UVS

#else
#ifdef PBR_DEBUG_DEPTH
    return vec4<f32>(in.frag_coord.z, in.frag_coord.z, in.frag_coord.z, 1.0);
#else
#ifdef PBR_DEBUG_INTERPOLATED_VERTEX_NORMALS
    return vec4<f32>(in.world_normal, 1.0);
#else
#ifdef PBR_DEBUG_INTERPOLATED_VERTEX_TANGENTS

#ifdef VERTEX_TANGENTS
    return vec4<f32>(in.world_tangent.rgb, 1.0);
#else
    return pink;
#endif // VERTEX_TANGENTS

#else
#ifdef PBR_DEBUG_TANGENT_SPACE_NORMAL_MAP

#ifdef VERTEX_UVS
#ifdef STANDARDMATERIAL_NORMAL_MAP
    let Nt = prepare_tangent_space_normal(material.flags, in.uv);
    return vec4<f32>(Nt, 1.0);
#else
    return pink;
#endif // STANDARDMATERIAL_NORMAL_MAP
#else
    return pink;
#endif // VERTEX_UVS

#else
#ifdef PBR_DEBUG_NORMAL_MAPPED_NORMAL

#ifdef VERTEX_UVS
#ifdef VERTEX_TANGENTS
#ifdef STANDARDMATERIAL_NORMAL_MAP
    let N = prepare_normal(
        material.flags,
        in.world_normal,
        in.world_tangent,
        in.uv,
        in.is_front,
    );
    return vec4<f32>(N, 1.0);
#else
    return pink;
#endif // STANDARDMATERIAL_NORMAL_MAP
#else
    return pink;
#endif // VERTEX_TANGENTS
#else
    return pink;
#endif // VERTEX_UVS


#else
#ifdef PBR_DEBUG_VIEW_SPACE_NORMAL_MAPPED_NORMAL

#ifdef VERTEX_UVS
#ifdef VERTEX_TANGENTS
#ifdef STANDARDMATERIAL_NORMAL_MAP
    var N: vec3<f32> = prepare_normal(
        material.flags,
        in.world_normal,
        in.world_tangent,
        in.uv,
        in.is_front,
    );
    // Normals should be transformed by the inverse transpose of the usual transform applied to a
    // vertex. The 'forward' transform is the inverse view transform. So in this case we want the
    // inverse transpose of the inverse view transform, which is the transpose of the view
    // transform.
    N = transpose(mat3x3<f32>(
        view.view.x.xyz,
        view.view.y.xyz,
        view.view.z.xyz
    )) * N;
    return vec4<f32>(N, 1.0);
#else
    return pink;
#endif // STANDARDMATERIAL_NORMAL_MAP
#else
    return pink;
#endif // VERTEX_TANGENTS
#else
    return pink;
#endif // VERTEX_UVS

#else
#ifdef PBR_DEBUG_BASE_COLOR
    return material.base_color;
#else
#ifdef PBR_DEBUG_BASE_COLOR_TEXTURE

#ifdef VERTEX_UVS
    if ((material.flags & STANDARD_MATERIAL_FLAGS_BASE_COLOR_TEXTURE_BIT) != 0u) {
        return textureSample(base_color_texture, base_color_sampler, in.uv);
    } else {
        return pink;
    }
#else
    return pink;
#endif // VERTEX_UVS

#else
#ifdef PBR_DEBUG_EMISSIVE
    return material.emissive;
#else
#ifdef PBR_DEBUG_EMISSIVE_TEXTURE

#ifdef VERTEX_UVS
    if ((material.flags & STANDARD_MATERIAL_FLAGS_EMISSIVE_TEXTURE_BIT) != 0u) {
        return vec4<f32>(
            textureSample(emissive_texture, emissive_sampler, in.uv).rgb,
            1.0
        );
    } else {
        return pink;
    }
#else
    return pink;
#endif // VERTEX_UVS

#else
#ifdef PBR_DEBUG_ROUGHNESS
    return vec4<f32>(
        material.perceptual_roughness,
        material.perceptual_roughness,
        material.perceptual_roughness,
        1.0
    );
#else
#ifdef PBR_DEBUG_ROUGHNESS_TEXTURE

#ifdef VERTEX_UVS
    if ((material.flags & STANDARD_MATERIAL_FLAGS_METALLIC_ROUGHNESS_TEXTURE_BIT) != 0u) {
        let perceptual_roughness = textureSample(metallic_roughness_texture, metallic_roughness_sampler, in.uv).g;
        return vec4<f32>(
            perceptual_roughness,
            perceptual_roughness,
            perceptual_roughness,
            1.0
        );
    } else {
        return pink;
    }
#else
    return pink;
#endif // VERTEX_UVS

#else
#ifdef PBR_DEBUG_METALLIC
    return vec4<f32>(
        material.metallic,
        material.metallic,
        material.metallic,
        1.0
    );
#else
#ifdef PBR_DEBUG_METALLIC_TEXTURE

#ifdef VERTEX_UVS
    if ((material.flags & STANDARD_MATERIAL_FLAGS_METALLIC_ROUGHNESS_TEXTURE_BIT) != 0u) {
        let metallic = textureSample(metallic_roughness_texture, metallic_roughness_sampler, in.uv).b;
        return vec4<f32>(
            metallic,
            metallic,
            metallic,
            1.0
        );
    } else {
        return pink;
    }
#else
    return pink;
#endif // VERTEX_UVS

#else
#ifdef PBR_DEBUG_REFLECTANCE
    return vec4<f32>(
        material.reflectance,
        material.reflectance,
        material.reflectance,
        1.0
    );
#else
#ifdef PBR_DEBUG_OCCLUSION_TEXTURE

#ifdef VERTEX_UVS
    if ((material.flags & STANDARD_MATERIAL_FLAGS_OCCLUSION_TEXTURE_BIT) != 0u) {
        let occlusion = textureSample(occlusion_texture, occlusion_sampler, in.uv).r;
        return vec4<f32>(
            occlusion,
            occlusion,
            occlusion,
            1.0
        );
    } else {
        return pink;
    }
#else
    return pink;
#endif // VERTEX_UVS

#endif // PBR_DEBUG_OCCLUSION_TEXTURE
#endif // PBR_DEBUG_REFLECTANCE
#endif // PBR_DEBUG_METALLIC_TEXTURE
#endif // PBR_DEBUG_METALLIC
#endif // PBR_DEBUG_ROUGHNESS_TEXTURE
#endif // PBR_DEBUG_ROUGHNESS
#endif // PBR_DEBUG_EMISSIVE_TEXTURE
#endif // PBR_DEBUG_EMISSIVE
#endif // PBR_DEBUG_BASE_COLOR_TEXTURE
#endif // PBR_DEBUG_BASE_COLOR
#endif // PBR_DEBUG_VIEW_SPACE_NORMAL_MAPPED_NORMAL
#endif // PBR_DEBUG_NORMAL_MAPPED_NORMAL
#endif // PBR_DEBUG_TANGENT_SPACE_NORMAL_MAP
#endif // PBR_DEBUG_INTERPOLATED_VERTEX_TANGENTS
#endif // PBR_DEBUG_INTERPOLATED_VERTEX_NORMALS
#endif // PBR_DEBUG_DEPTH
#endif // PBR_DEBUG_UVS
}

#endif // PBR_DEBUG
