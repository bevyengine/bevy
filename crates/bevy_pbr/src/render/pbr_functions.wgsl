#define_import_path bevy_pbr::pbr_functions

#ifdef ENVIRONMENT_MAP
#import bevy_pbr::environment_map
#endif

fn direct_lighting(s: PbrState) -> vec3<f32> {
    let world_position = s.in.world_position;
    let world_normal = s.in.world_normal;

    // Determine lights within pixel's cluster
    let view_z = dot(vec4<f32>(
        view.inverse_view[0].z,
        view.inverse_view[1].z,
        view.inverse_view[2].z,
        view.inverse_view[3].z
    ), s.in.world_position);
    let cluster_index = fragment_cluster_index(s.in.frag_coord.xy, view_z, s.in.is_orthographic);
    let offset_and_counts = unpack_offset_and_counts(cluster_index);

    var direct_light = vec3(0.0);

    // Point lights
    for (var i: u32 = offset_and_counts[0]; i < offset_and_counts[0] + offset_and_counts[1]; i = i + 1u) {
        let light_id = get_light_id(i);
        let light = point_lights.data[light_id];
        var shadow = 1.0;
        if (mesh.flags & MESH_FLAGS_SHADOW_RECEIVER_BIT) != 0u && (light.flags & POINT_LIGHT_FLAGS_SHADOWS_ENABLED_BIT) != 0u {
            shadow = fetch_point_shadow(light_id, world_position, world_normal);
        }
        direct_light += point_light(light, s) * shadow;
    }

    // Spot lights
    for (var i: u32 = offset_and_counts[0] + offset_and_counts[1]; i < offset_and_counts[0] + offset_and_counts[1] + offset_and_counts[2]; i = i + 1u) {
        let light_id = get_light_id(i);
        let light = point_lights.data[light_id];
        var shadow = 1.0;
        if (mesh.flags & MESH_FLAGS_SHADOW_RECEIVER_BIT) != 0u && (light.flags & POINT_LIGHT_FLAGS_SHADOWS_ENABLED_BIT) != 0u {
            shadow = fetch_spot_shadow(light_id, world_position, world_normal);
        }
        direct_light += spot_light(light, s) * shadow;
    }

    // Directional lights
    let n_directional_lights = lights.n_directional_lights;
    for (var i: u32 = 0u; i < n_directional_lights; i = i + 1u) {
        let light = lights.directional_lights[i];
        var shadow = 1.0;
        if (mesh.flags & MESH_FLAGS_SHADOW_RECEIVER_BIT) != 0u && (light.flags & DIRECTIONAL_LIGHT_FLAGS_SHADOWS_ENABLED_BIT) != 0u {
            shadow = fetch_directional_shadow(i, world_position, world_normal);
        }
        direct_light += directional_light(light, s) * shadow;
    }

    return direct_light;
}

fn indirect_lighting(s: PbrState) -> vec3<f32> {
    var indirect_diffuse_light = vec3(0.0);
    var indirect_specular_light = vec3(0.0);

    // Ambient light
    indirect_diffuse_light += EnvBRDFApprox(s.diffuse_color, F_AB(1.0, s.NdotV)) * lights.ambient_color.rgb;
    indirect_specular_light += EnvBRDFApprox(s.F0, s.f_ab) * lights.ambient_color.rgb;

    // Environment map light
#ifdef ENVIRONMENT_MAP
    let environment_light = environment_map_light(s);
    indirect_diffuse_light += environment_light.diffuse;
    indirect_specular_light += environment_light.specular;
#endif

    // Apply indirect occlusion
    indirect_diffuse_light *= s.in.occlusion;

    // Combine diffuse and specular light
    return indirect_diffuse_light + indirect_specular_light;
}

fn perceptualRoughnessToRoughness(perceptualRoughness: f32) -> f32 {
    // clamp perceptual roughness to prevent precision problems
    // According to Filament design 0.089 is recommended for mobile
    // Filament uses 0.045 for non-mobile
    let clampedPerceptualRoughness = clamp(perceptualRoughness, 0.089, 1.0);
    return clampedPerceptualRoughness * clampedPerceptualRoughness;
}

fn pbr(
    in: PbrInput,
) -> vec4<f32> {
    let material = in.material;

    // Setup needed state
    var s: PbrState;
    s.in = in;

    // Convert perceptually-linear roughness to actual roughness
    s.roughness = perceptualRoughnessToRoughness(material.perceptual_roughness);
    s.clear_coat_roughness = perceptualRoughnessToRoughness(material.clear_coat_perceptual_roughness);

    // Diffuse strength inversely related to metallicity
    s.diffuse_color = material.base_color.rgb * (1.0 - material.metallic);

    // Neubelt and Pettineo 2013, "Crafting a Next-gen Material Pipeline for The Order: 1886"
    s.NdotV = max(dot(in.N, in.V), 0.0001);
    s.R = reflect(-in.V, in.N);

    // Remapping [0,1] reflectance to F0
    // See https://google.github.io/filament/Filament.html#materialsystem/parameterization/remapping
    s.F0 = 0.16 * material.reflectance * material.reflectance * (1.0 - material.metallic) + material.base_color.rgb * material.metallic;

    // Scale and bias used for some forms of indirect lighting
    s.f_ab = F_AB(material.perceptual_roughness, s.NdotV);

    // Calculate lighting
    let direct_light = direct_lighting(s);
    let indirect_light = indirect_lighting(s);
    let emissive_light = material.emissive.rgb * material.base_color.a;
    let total_light = direct_light + indirect_light + emissive_light;

    var output_color = vec4<f32>(total_light, material.base_color.a);
    // output_color = cluster_debug_visualization(
    //     output_color,
    //     view_z,
    //     in.is_orthographic,
    //     offset_and_counts,
    //     cluster_index,
    // );

    return output_color;
}

fn alpha_discard(material: StandardMaterial, output_color: vec4<f32>) -> vec4<f32> {
    var color = output_color;
    if (material.flags & STANDARD_MATERIAL_FLAGS_ALPHA_MODE_OPAQUE) != 0u {
        // NOTE: If rendering as opaque, alpha should be ignored so set to 1.0
        color.a = 1.0;
    } else if (material.flags & STANDARD_MATERIAL_FLAGS_ALPHA_MODE_MASK) != 0u {
        if color.a >= material.alpha_cutoff {
            // NOTE: If rendering as masked alpha and >= the cutoff, render as fully opaque
            color.a = 1.0;
        } else {
            // NOTE: output_color.a < in.material.alpha_cutoff should not is not rendered
            // NOTE: This and any other discards mean that early-z testing cannot be done!
            discard;
        }
    }
    return color;
}

fn prepare_world_normal(
    world_normal: vec3<f32>,
    double_sided: bool,
    is_front: bool,
) -> vec3<f32> {
    var output: vec3<f32> = world_normal;
    #ifndef VERTEX_TANGENTS
    #ifndef STANDARDMATERIAL_NORMAL_MAP
    // NOTE: When NOT using normal-mapping, if looking at the back face of a double-sided
    // material, the normal needs to be inverted. This is a branchless version of that.
    output = (f32(!double_sided || is_front) * 2.0 - 1.0) * output;
#endif
#endif
    return output;
}

fn apply_normal_mapping(
    standard_material_flags: u32,
    world_normal: vec3<f32>,
#ifdef VERTEX_TANGENTS
#ifdef STANDARDMATERIAL_NORMAL_MAP
    world_tangent: vec4<f32>,
#endif
#endif
#ifdef VERTEX_UVS
    uv: vec2<f32>,
#endif
) -> vec3<f32> {
    // NOTE: The mikktspace method of normal mapping explicitly requires that the world normal NOT
    // be re-normalized in the fragment shader. This is primarily to match the way mikktspace
    // bakes vertex tangents and normal maps so that this is the exact inverse. Blender, Unity,
    // Unreal Engine, Godot, and more all use the mikktspace method. Do not change this code
    // unless you really know what you are doing.
    // http://www.mikktspace.com/
    var N: vec3<f32> = world_normal;

#ifdef VERTEX_TANGENTS
#ifdef STANDARDMATERIAL_NORMAL_MAP
    // NOTE: The mikktspace method of normal mapping explicitly requires that these NOT be
    // normalized nor any Gram-Schmidt applied to ensure the vertex normal is orthogonal to the
    // vertex tangent! Do not change this code unless you really know what you are doing.
    // http://www.mikktspace.com/
    var T: vec3<f32> = world_tangent.xyz;
    var B: vec3<f32> = world_tangent.w * cross(N, T);
#endif
#endif

#ifdef VERTEX_TANGENTS
#ifdef VERTEX_UVS
#ifdef STANDARDMATERIAL_NORMAL_MAP
    // Nt is the tangent-space normal.
    var Nt = textureSample(normal_map_texture, normal_map_sampler, uv).rgb;
    if (standard_material_flags & STANDARD_MATERIAL_FLAGS_TWO_COMPONENT_NORMAL_MAP) != 0u {
        // Only use the xy components and derive z for 2-component normal maps.
        Nt = vec3<f32>(Nt.rg * 2.0 - 1.0, 0.0);
        Nt.z = sqrt(1.0 - Nt.x * Nt.x - Nt.y * Nt.y);
    } else {
        Nt = Nt * 2.0 - 1.0;
    }
    // Normal maps authored for DirectX require flipping the y component
    if (standard_material_flags & STANDARD_MATERIAL_FLAGS_FLIP_NORMAL_MAP_Y) != 0u {
        Nt.y = -Nt.y;
    }
    // NOTE: The mikktspace method of normal mapping applies maps the tangent-space normal from
    // the normal map texture in this way to be an EXACT inverse of how the normal map baker
    // calculates the normal maps so there is no error introduced. Do not change this code
    // unless you really know what you are doing.
    // http://www.mikktspace.com/
    N = Nt.x * T + Nt.y * B + Nt.z * N;
#endif
#endif
#endif

    return normalize(N);
}

// NOTE: Correctly calculates the view vector depending on whether
// the projection is orthographic or perspective.
fn calculate_view(
    world_position: vec4<f32>,
    is_orthographic: bool,
) -> vec3<f32> {
    var V: vec3<f32>;
    if is_orthographic {
        // Orthographic view vector
        V = normalize(vec3<f32>(view.view_proj[0].z, view.view_proj[1].z, view.view_proj[2].z));
    } else {
        // Only valid for a perpective projection
        V = normalize(view.world_position.xyz - world_position.xyz);
    }
    return V;
}
