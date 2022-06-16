#import bevy_pbr::mesh_view_bindings
#import bevy_pbr::pbr_bindings
#import bevy_pbr::mesh_bindings

#import bevy_pbr::utils
#import bevy_pbr::clustered_forward
#import bevy_pbr::lighting
#import bevy_pbr::shadows

struct FragmentInput {
    [[builtin(front_facing)]] is_front: bool;
    [[builtin(position)]] frag_coord: vec4<f32>;
    [[location(0)]] world_position: vec4<f32>;
    [[location(1)]] world_normal: vec3<f32>;
    [[location(2)]] uv: vec2<f32>;
#ifdef VERTEX_TANGENTS
    [[location(3)]] world_tangent: vec4<f32>;
#endif
#ifdef VERTEX_COLORS
    [[location(4)]] color: vec4<f32>;
#endif
};

[[stage(fragment)]]
fn fragment(in: FragmentInput) -> [[location(0)]] vec4<f32> {
    var output_color: vec4<f32> = material.base_color;
    #ifdef VERTEX_COLORS
    output_color = output_color * in.color;
    #endif
    if ((material.flags & STANDARD_MATERIAL_FLAGS_BASE_COLOR_TEXTURE_BIT) != 0u) {
        output_color = output_color * textureSample(base_color_texture, base_color_sampler, in.uv);
    }

    // // NOTE: Unlit bit not set means == 0 is true, so the true case is if lit
    if ((material.flags & STANDARD_MATERIAL_FLAGS_UNLIT_BIT) == 0u) {
        // TODO use .a for exposure compensation in HDR
        var emissive: vec4<f32> = material.emissive;
        if ((material.flags & STANDARD_MATERIAL_FLAGS_EMISSIVE_TEXTURE_BIT) != 0u) {
            emissive = vec4<f32>(emissive.rgb * textureSample(emissive_texture, emissive_sampler, in.uv).rgb, 1.0);
        }

        // calculate non-linear roughness from linear perceptualRoughness
        var metallic: f32 = material.metallic;
        var perceptual_roughness: f32 = material.perceptual_roughness;
        if ((material.flags & STANDARD_MATERIAL_FLAGS_METALLIC_ROUGHNESS_TEXTURE_BIT) != 0u) {
            let metallic_roughness = textureSample(metallic_roughness_texture, metallic_roughness_sampler, in.uv);
            // Sampling from GLTF standard channels for now
            metallic = metallic * metallic_roughness.b;
            perceptual_roughness = perceptual_roughness * metallic_roughness.g;
        }
        let roughness = perceptualRoughnessToRoughness(perceptual_roughness);

        var occlusion: f32 = 1.0;
        if ((material.flags & STANDARD_MATERIAL_FLAGS_OCCLUSION_TEXTURE_BIT) != 0u) {
            occlusion = textureSample(occlusion_texture, occlusion_sampler, in.uv).r;
        }

        var N: vec3<f32> = normalize(in.world_normal);

#ifdef VERTEX_TANGENTS
#ifdef STANDARDMATERIAL_NORMAL_MAP
        // NOTE: The mikktspace method of normal mapping explicitly requires that these NOT be
        // normalized nor any Gram-Schmidt applied to ensure the vertex normal is orthogonal to the
        // vertex tangent! Do not change this code unless you really know what you are doing.
        // http://www.mikktspace.com/
        var T: vec3<f32> = in.world_tangent.xyz;
        var B: vec3<f32> = in.world_tangent.w * cross(N, T);
#endif
#endif

        if ((material.flags & STANDARD_MATERIAL_FLAGS_DOUBLE_SIDED_BIT) != 0u) {
            if (!in.is_front) {
                N = -N;
#ifdef VERTEX_TANGENTS
#ifdef STANDARDMATERIAL_NORMAL_MAP
                T = -T;
                B = -B;
#endif
#endif
            }
        }

#ifdef VERTEX_TANGENTS
#ifdef STANDARDMATERIAL_NORMAL_MAP
        let TBN = mat3x3<f32>(T, B, N);
        // Nt is the tangent-space normal.
        var Nt: vec3<f32>;
        if ((material.flags & STANDARD_MATERIAL_FLAGS_TWO_COMPONENT_NORMAL_MAP) != 0u) {
            // Only use the xy components and derive z for 2-component normal maps.
            Nt = vec3<f32>(textureSample(normal_map_texture, normal_map_sampler, in.uv).rg * 2.0 - 1.0, 0.0);
            Nt.z = sqrt(1.0 - Nt.x * Nt.x - Nt.y * Nt.y);
        } else {
            Nt = textureSample(normal_map_texture, normal_map_sampler, in.uv).rgb * 2.0 - 1.0;
        }
        // Normal maps authored for DirectX require flipping the y component
        if ((material.flags & STANDARD_MATERIAL_FLAGS_FLIP_NORMAL_MAP_Y) != 0u) {
            Nt.y = -Nt.y;
        }
        // NOTE: The mikktspace method of normal mapping applies maps the tangent-space normal from
        // the normal map texture in this way to be an EXACT inverse of how the normal map baker
        // calculates the normal maps so there is no error introduced. Do not change this code
        // unless you really know what you are doing.
        // http://www.mikktspace.com/
        N = normalize(Nt.x * T + Nt.y * B + Nt.z * N);
#endif
#endif

        if ((material.flags & STANDARD_MATERIAL_FLAGS_ALPHA_MODE_OPAQUE) != 0u) {
            // NOTE: If rendering as opaque, alpha should be ignored so set to 1.0
            output_color.a = 1.0;
        } else if ((material.flags & STANDARD_MATERIAL_FLAGS_ALPHA_MODE_MASK) != 0u) {
            if (output_color.a >= material.alpha_cutoff) {
                // NOTE: If rendering as masked alpha and >= the cutoff, render as fully opaque
                output_color.a = 1.0;
            } else {
                // NOTE: output_color.a < material.alpha_cutoff should not is not rendered
                // NOTE: This and any other discards mean that early-z testing cannot be done!
                discard;
            }
        }

        var V: vec3<f32>;
        // If the projection is not orthographic
        let is_orthographic = view.projection[3].w == 1.0;
        if (is_orthographic) {
            // Orthographic view vector
            V = normalize(vec3<f32>(view.view_proj[0].z, view.view_proj[1].z, view.view_proj[2].z));
        } else {
            // Only valid for a perpective projection
            V = normalize(view.world_position.xyz - in.world_position.xyz);
        }

        // Neubelt and Pettineo 2013, "Crafting a Next-gen Material Pipeline for The Order: 1886"
        let NdotV = max(dot(N, V), 0.0001);

        // Remapping [0,1] reflectance to F0
        // See https://google.github.io/filament/Filament.html#materialsystem/parameterization/remapping
        let reflectance = material.reflectance;
        let F0 = 0.16 * reflectance * reflectance * (1.0 - metallic) + output_color.rgb * metallic;

        // Diffuse strength inversely related to metallicity
        let diffuse_color = output_color.rgb * (1.0 - metallic);

        let R = reflect(-V, N);

        // accumulate color
        var light_accum: vec3<f32> = vec3<f32>(0.0);

        let view_z = dot(vec4<f32>(
            view.inverse_view[0].z,
            view.inverse_view[1].z,
            view.inverse_view[2].z,
            view.inverse_view[3].z
        ), in.world_position);
        let cluster_index = fragment_cluster_index(in.frag_coord.xy, view_z, is_orthographic);
        let offset_and_count = unpack_offset_and_count(cluster_index);
        for (var i: u32 = offset_and_count[0]; i < offset_and_count[0] + offset_and_count[1]; i = i + 1u) {
            let light_id = get_light_id(i);
            let light = point_lights.data[light_id];
            var shadow: f32 = 1.0;
            if ((mesh.flags & MESH_FLAGS_SHADOW_RECEIVER_BIT) != 0u
                    && (light.flags & POINT_LIGHT_FLAGS_SHADOWS_ENABLED_BIT) != 0u) {
                shadow = fetch_point_shadow(light_id, in.world_position, in.world_normal);
            }
            let light_contrib = point_light(in.world_position.xyz, light, roughness, NdotV, N, V, R, F0, diffuse_color);
            light_accum = light_accum + light_contrib * shadow;
        }

        let n_directional_lights = lights.n_directional_lights;
        for (var i: u32 = 0u; i < n_directional_lights; i = i + 1u) {
            let light = lights.directional_lights[i];
            var shadow: f32 = 1.0;
            if ((mesh.flags & MESH_FLAGS_SHADOW_RECEIVER_BIT) != 0u
                    && (light.flags & DIRECTIONAL_LIGHT_FLAGS_SHADOWS_ENABLED_BIT) != 0u) {
                shadow = fetch_directional_shadow(i, in.world_position, in.world_normal);
            }
            let light_contrib = directional_light(light, roughness, NdotV, N, V, R, F0, diffuse_color);
            light_accum = light_accum + light_contrib * shadow;
        }

        let diffuse_ambient = EnvBRDFApprox(diffuse_color, 1.0, NdotV);
        let specular_ambient = EnvBRDFApprox(F0, perceptual_roughness, NdotV);

        output_color = vec4<f32>(
            light_accum +
                (diffuse_ambient + specular_ambient) * lights.ambient_color.rgb * occlusion +
                emissive.rgb * output_color.a,
            output_color.a);

        output_color = cluster_debug_visualization(
            output_color,
            view_z,
            is_orthographic,
            offset_and_count,
            cluster_index,
        );

        // tone_mapping
        output_color = vec4<f32>(reinhard_luminance(output_color.rgb), output_color.a);
        // Gamma correction.
        // Not needed with sRGB buffer
        // output_color.rgb = pow(output_color.rgb, vec3(1.0 / 2.2));
    }

    return output_color;
}
