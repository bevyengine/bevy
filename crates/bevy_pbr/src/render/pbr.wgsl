#import bevy_pbr::mesh_view_bindings
#import bevy_pbr::pbr_bindings
#import bevy_pbr::mesh_bindings

#import bevy_pbr::utils
#import bevy_pbr::clustered_forward
#import bevy_pbr::lighting
#import bevy_pbr::shadows
#import bevy_pbr::pbr_functions

#ifdef TONEMAP_IN_SHADER
#import bevy_core_pipeline::tonemapping
#endif

struct FragmentInput {
    @builtin(front_facing) is_front: bool,
    @builtin(position) frag_coord: vec4<f32>,
    #import bevy_pbr::mesh_vertex_output
};

@fragment
fn fragment(in: FragmentInput) -> @location(0) vec4<f32> {
    // Load base material color
    var base_color: vec4<f32> = material.base_color;
#ifdef VERTEX_COLORS
    base_color *= in.color;
#endif
#ifdef VERTEX_UVS
    if (material.flags & STANDARD_MATERIAL_FLAGS_BASE_COLOR_TEXTURE_BIT) != 0u {
        base_color *= textureSample(base_color_texture, base_color_sampler, in.uv);
    }
#endif

    // Switch on whether to apply PBR shading or not (unlit vs shaded)
    // NOTE: Unlit bit not set means == 0 is true, so the true case is if lit
    var output_color: vec4<f32>;
    if (material.flags & STANDARD_MATERIAL_FLAGS_UNLIT_BIT) == 0u {
        // Prepare PbrInput
        var pbr_input: PbrInput;
        pbr_input.material = material;

        // Combine all material fields with their texture variants
#ifdef VERTEX_UVS
        // TODO use alpha for exposure compensation in HDR
        if (material.flags & STANDARD_MATERIAL_FLAGS_EMISSIVE_TEXTURE_BIT) != 0u {
            let emissive = vec4<f32>(textureSample(emissive_texture, emissive_sampler, in.uv).rgb, 1.0);
            pbr_input.material.emissive *= emissive;
        }

        if (material.flags & STANDARD_MATERIAL_FLAGS_METALLIC_ROUGHNESS_TEXTURE_BIT) != 0u {
            let metallic_roughness = textureSample(metallic_roughness_texture, metallic_roughness_sampler, in.uv);
            // Sampling from GLTF standard channels for now
            pbr_input.material.metallic *= metallic_roughness.b;
            pbr_input.material.perceptual_roughness *= metallic_roughness.g;
        }

        if (material.flags & STANDARD_MATERIAL_FLAGS_OCCLUSION_TEXTURE_BIT) != 0u {
            pbr_input.occlusion = textureSample(occlusion_texture, occlusion_sampler, in.uv).r;
        } else {
            pbr_input.occlusion = 1.0;
        }
#endif

        // Apply alpha masking - needs to be done after all textureSamples according to WGSL
        pbr_input.material.base_color = alpha_discard(material, base_color);

        // Calculate view vector, normal vector, world position, etc
        pbr_input.frag_coord = in.frag_coord;
        pbr_input.world_position = in.world_position;
        pbr_input.world_normal = prepare_world_normal(
            in.world_normal,
            (material.flags & STANDARD_MATERIAL_FLAGS_DOUBLE_SIDED_BIT) != 0u,
            in.is_front,
        );

        pbr_input.is_orthographic = view.projection[3].w == 1.0;

        pbr_input.N = apply_normal_mapping(
            material.flags,
            pbr_input.world_normal,
#ifdef VERTEX_TANGENTS
#ifdef STANDARDMATERIAL_NORMAL_MAP
            in.world_tangent,
#endif
#endif
#ifdef VERTEX_UVS
            in.uv,
#endif
        );
        pbr_input.V = calculate_view(in.world_position, pbr_input.is_orthographic);

        // Apply PBR shading
        output_color = pbr(pbr_input);
    } else {
        // Unlit - no shading, just base color + alpha masking
        output_color = alpha_discard(material, base_color);
    }

// If not separately tonemapping in a later render pass, do so here
#ifdef TONEMAP_IN_SHADER
    var output_rgb = reinhard_luminance(output_color.rgb);

#ifdef DEBAND_DITHER
    output_rgb = pow(output_rgb, vec3<f32>(1.0 / 2.2));
    output_rgb = output_rgb + screen_space_dither(in.frag_coord.xy);
    // This conversion back to linear space is required because our output texture format is
    // SRGB; the GPU will assume our output is linear and will apply an SRGB conversion.
    output_rgb = pow(output_rgb, vec3<f32>(2.2));
#endif

    output_color = vec4<f32>(output_rgb, output_color.a);
#endif

    return output_color;
}
