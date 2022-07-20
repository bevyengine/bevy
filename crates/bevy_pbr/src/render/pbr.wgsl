#import bevy_pbr::mesh_view_bindings
#import bevy_pbr::pbr_bindings
#import bevy_pbr::mesh_bindings

#import bevy_pbr::utils
#import bevy_pbr::clustered_forward
#import bevy_pbr::lighting
#import bevy_pbr::shadows
#import bevy_pbr::pbr_functions

struct FragmentInput {
    @builtin(front_facing) is_front: bool,
    @builtin(position) frag_coord: vec4<f32>,
    #import bevy_pbr::mesh_vertex_output
};

// NOTE: bevy_pbr::pbr_debug needs specific things from the fragment input and standard material
#import bevy_pbr::pbr_debug

@fragment
fn fragment(in: FragmentInput) -> @location(0) vec4<f32> {
#ifdef PBR_DEBUG
    return pbr_debug(in);
#else
#ifdef PBR_DEBUG_OPAQUE
    if ((material.flags & STANDARD_MATERIAL_FLAGS_ALPHA_MODE_MASK) != 0u
            || (material.flags & STANDARD_MATERIAL_FLAGS_ALPHA_MODE_BLEND) != 0u) {
        discard;
    }
#endif
#ifdef PBR_DEBUG_ALPHA_MASK
    if ((material.flags & STANDARD_MATERIAL_FLAGS_ALPHA_MODE_OPAQUE) != 0u
            || (material.flags & STANDARD_MATERIAL_FLAGS_ALPHA_MODE_BLEND) != 0u) {
        discard;
    }
#endif
#ifdef PBR_DEBUG_ALPHA_BLEND
    if ((material.flags & STANDARD_MATERIAL_FLAGS_ALPHA_MODE_OPAQUE) != 0u
            || (material.flags & STANDARD_MATERIAL_FLAGS_ALPHA_MODE_MASK) != 0u) {
        discard;
    }
#endif

    var output_color: vec4<f32> = material.base_color;
#ifdef VERTEX_COLORS
    output_color = output_color * in.color;
#endif // VERTEX_COLORS
#ifdef VERTEX_UVS
    if ((material.flags & STANDARD_MATERIAL_FLAGS_BASE_COLOR_TEXTURE_BIT) != 0u) {
        output_color = output_color * textureSample(base_color_texture, base_color_sampler, in.uv);
    }
#endif // VERTEX_UVS

    // NOTE: Unlit bit not set means == 0 is true, so the true case is if lit
    if ((material.flags & STANDARD_MATERIAL_FLAGS_UNLIT_BIT) == 0u) {
        // Prepare a 'processed' StandardMaterial by sampling all textures to resolve
        // the material members
        var pbr_input: PbrInput;

        pbr_input.material.base_color = output_color;
        pbr_input.material.reflectance = material.reflectance;
        pbr_input.material.flags = material.flags;
        pbr_input.material.alpha_cutoff = material.alpha_cutoff;

        // TODO use .a for exposure compensation in HDR
        var emissive: vec4<f32> = material.emissive;
#ifdef VERTEX_UVS
        if ((material.flags & STANDARD_MATERIAL_FLAGS_EMISSIVE_TEXTURE_BIT) != 0u) {
            emissive = vec4<f32>(emissive.rgb * textureSample(emissive_texture, emissive_sampler, in.uv).rgb, 1.0);
        }
#endif // VERTEX_UVS
        pbr_input.material.emissive = emissive;

        var metallic: f32 = material.metallic;
        var perceptual_roughness: f32 = material.perceptual_roughness;
#ifdef VERTEX_UVS
        if ((material.flags & STANDARD_MATERIAL_FLAGS_METALLIC_ROUGHNESS_TEXTURE_BIT) != 0u) {
            let metallic_roughness = textureSample(metallic_roughness_texture, metallic_roughness_sampler, in.uv);
            // Sampling from GLTF standard channels for now
            metallic = metallic * metallic_roughness.b;
            perceptual_roughness = perceptual_roughness * metallic_roughness.g;
        }
#endif // VERTEX_UVS
        pbr_input.material.metallic = metallic;
        pbr_input.material.perceptual_roughness = perceptual_roughness;

        var occlusion: f32 = 1.0;
#ifdef VERTEX_UVS
        if ((material.flags & STANDARD_MATERIAL_FLAGS_OCCLUSION_TEXTURE_BIT) != 0u) {
            occlusion = textureSample(occlusion_texture, occlusion_sampler, in.uv).r;
        }
#endif // VERTEX_UVS
        pbr_input.occlusion = occlusion;

        pbr_input.frag_coord = in.frag_coord;
        pbr_input.world_position = in.world_position;
        pbr_input.world_normal = in.world_normal;

        pbr_input.is_orthographic = view.projection[3].w == 1.0;

        pbr_input.N = prepare_normal(
            material.flags,
            in.world_normal,
#ifdef VERTEX_TANGENTS
#ifdef STANDARDMATERIAL_NORMAL_MAP
            in.world_tangent,
#endif // STANDARDMATERIAL_NORMAL_MAP
#endif // VERTEX_TANGENTS
#ifdef VERTEX_UVS
            in.uv,
#endif // VERTEX_UVS
            in.is_front,
        );
        pbr_input.V = calculate_view(in.world_position, pbr_input.is_orthographic);

        output_color = tone_mapping(pbr(pbr_input));
    }

    return output_color;
#endif // PBR_DEBUG
}
