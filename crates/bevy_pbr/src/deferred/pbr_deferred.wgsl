#define_import_path bevy_pbr::pbr_deferred

struct StandardPbrDeferredOutput {
    deferred: vec4<u32>,
    normal: vec3<f32>,
}

// NOTE: KEEP IN FEATURE PARITY WITH @fragment IN pbr.wgsl
fn standard_pbr_deferred(in: FragmentInput) -> StandardPbrDeferredOutput {
    var out: StandardPbrDeferredOutput;
    var pbr_input: PbrInput;
    var deferred = vec4(0u);
    out.normal = in.world_normal;

    let is_orthographic = view.projection[3].w == 1.0;
    let V = calculate_view(in.world_position, is_orthographic);
#ifdef VERTEX_UVS
    var uv = in.uv;
#ifdef VERTEX_TANGENTS
    if ((material.flags & STANDARD_MATERIAL_FLAGS_DEPTH_MAP_BIT) != 0u) {
        let N = in.world_normal;
        let T = in.world_tangent.xyz;
        let B = in.world_tangent.w * cross(N, T);
        // Transform V from fragment to camera in world space to tangent space.
        let Vt = vec3(dot(V, T), dot(V, B), dot(V, N));
        uv = parallaxed_uv(
            material.parallax_depth_scale,
            material.max_parallax_layer_count,
            material.max_relief_mapping_search_steps,
            uv,
            // Flip the direction of Vt to go toward the surface to make the
            // parallax mapping algorithm easier to understand and reason
            // about.
            -Vt,
        );
    }
#endif
#endif
    var output_color: vec4<f32> = material.base_color;
#ifdef VERTEX_COLORS
    output_color = output_color * in.color;
#endif
#ifdef VERTEX_UVS
    if ((material.flags & STANDARD_MATERIAL_FLAGS_BASE_COLOR_TEXTURE_BIT) != 0u) {
        output_color = output_color * textureSample(base_color_texture, base_color_sampler, uv);
    }
#endif // VERTEX_UVS

    // NOTE: Unlit bit not set means == 0 is true, so the true case is if lit
    if ((material.flags & STANDARD_MATERIAL_FLAGS_UNLIT_BIT) == 0u) {
        // Prepare a 'processed' StandardMaterial by sampling all textures to resolve
        // the material members

        pbr_input.material.base_color = output_color;
        pbr_input.material.reflectance = material.reflectance;
        pbr_input.material.flags = material.flags;
        pbr_input.material.alpha_cutoff = material.alpha_cutoff;

        // TODO use .a for exposure compensation in HDR
        var emissive: vec4<f32> = material.emissive;
#ifdef VERTEX_UVS
        if ((material.flags & STANDARD_MATERIAL_FLAGS_EMISSIVE_TEXTURE_BIT) != 0u) {
            emissive = vec4<f32>(emissive.rgb * textureSample(emissive_texture, emissive_sampler, uv).rgb, 1.0);
        }
#endif
        pbr_input.material.emissive = emissive;

        var metallic: f32 = material.metallic;
        var perceptual_roughness: f32 = material.perceptual_roughness;
#ifdef VERTEX_UVS
        if ((material.flags & STANDARD_MATERIAL_FLAGS_METALLIC_ROUGHNESS_TEXTURE_BIT) != 0u) {
            let metallic_roughness = textureSample(metallic_roughness_texture, metallic_roughness_sampler, uv);
            // Sampling from GLTF standard channels for now
            metallic = metallic * metallic_roughness.b;
            perceptual_roughness = perceptual_roughness * metallic_roughness.g;
        }
#endif
        pbr_input.material.metallic = metallic;
        pbr_input.material.perceptual_roughness = perceptual_roughness;

        var occlusion: f32 = 1.0;
#ifdef VERTEX_UVS
        if ((material.flags & STANDARD_MATERIAL_FLAGS_OCCLUSION_TEXTURE_BIT) != 0u) {
            occlusion = textureSample(occlusion_texture, occlusion_sampler, uv).r;
        }
#endif
        pbr_input.frag_coord = in.frag_coord;
        pbr_input.world_position = in.world_position;

        pbr_input.world_normal = prepare_world_normal(
            in.world_normal,
            (material.flags & STANDARD_MATERIAL_FLAGS_DOUBLE_SIDED_BIT) != 0u,
            in.is_front,
        );

        pbr_input.is_orthographic = is_orthographic;

        pbr_input.N = apply_normal_mapping(
            material.flags,
            pbr_input.world_normal,
#ifdef VERTEX_TANGENTS
#ifdef STANDARDMATERIAL_NORMAL_MAP
            in.world_tangent,
#endif
#endif
#ifdef VERTEX_UVS
            uv,
#endif
        );
        pbr_input.V = V;
        pbr_input.occlusion = occlusion;

        var met_ref = pack_unorm4x8(vec4(
                pbr_input.material.metallic, 
                pbr_input.material.reflectance,
                //pbr_input.occlusion,
                0.0,
                0.0));

        let flags = deferred_flags_from_mesh_mat_flags(mesh.flags, pbr_input.material.flags);
        let oct_nor = octa_encode(normalize(pbr_input.N));
        let base_color_srgb = pow(pbr_input.material.base_color.rgb, vec3(1.0 / 2.2));
        out.deferred = vec4(
            pack_unorm4x8(vec4(base_color_srgb, pbr_input.material.perceptual_roughness)),
            float3_to_rgb9e5(pbr_input.material.emissive.rgb),
            pack_unorm1x16_onto_end(met_ref, in.frag_coord.z), // last 16 bytes are depth
            pack_24bit_nor_and_flags(oct_nor, flags),
        );
        out.normal = pbr_input.N;
    } else {
        let flags = deferred_flags_from_mesh_mat_flags(mesh.flags, 0u);
        let oct_nor = octa_encode(normalize(pbr_input.N));
        out.deferred = vec4(
            0u, 
            float3_to_rgb9e5(output_color.rgb), 
            0u, 
            pack_24bit_nor_and_flags(oct_nor, flags)
        );
    }

    return out;
}