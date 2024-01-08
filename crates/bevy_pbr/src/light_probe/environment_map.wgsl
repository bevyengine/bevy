#define_import_path bevy_pbr::environment_map

#import bevy_pbr::mesh_view_bindings as bindings
#import bevy_pbr::mesh_view_bindings::light_probes

struct EnvironmentMapLight {
    diffuse: vec3<f32>,
    specular: vec3<f32>,
};

struct EnvironmentMapRadiances {
    irradiance: vec3<f32>,
    radiance: vec3<f32>,
}

// Define two versions of this function, one for the case in which there are
// multiple light probes and one for the case in which only the view light probe
// is present.

#ifdef MULTIPLE_LIGHT_PROBES_IN_ARRAY

fn compute_radiances(
    perceptual_roughness: f32,
    N: vec3<f32>,
    R: vec3<f32>,
    world_position: vec3<f32>,
) -> EnvironmentMapRadiances {
    var radiances: EnvironmentMapRadiances;

    // Search for a reflection probe that contains the fragment.
    //
    // TODO: Interpolate between multiple reflection probes.
    var cubemap_index: i32 = -1;
    for (var reflection_probe_index: i32 = 0;
            reflection_probe_index < light_probes.reflection_probe_count;
            reflection_probe_index += 1) {
        let reflection_probe = light_probes.reflection_probes[reflection_probe_index];

        // Unpack the inverse transform.
        let inverse_transpose_transform = mat4x4<f32>(
            reflection_probe.inverse_transpose_transform[0],
            reflection_probe.inverse_transpose_transform[1],
            reflection_probe.inverse_transpose_transform[2],
            vec4<f32>(0.0, 0.0, 0.0, 1.0));
        let inverse_transform = transpose(inverse_transpose_transform);

        // Check to see if the transformed point is inside the unit cube
        // centered at the origin.
        let probe_space_pos = (inverse_transform * vec4<f32>(world_position, 1.0)).xyz;
        if (all(abs(probe_space_pos) <= vec3(0.5))) {
            cubemap_index = reflection_probe.cubemap_index;
            break;
        }
    }

    // If we didn't find a reflection probe, use the view environment map if applicable.
    if (cubemap_index < 0) {
        cubemap_index = light_probes.view_cubemap_index;
    }

    // If there's no cubemap, bail out.
    if (cubemap_index < 0) {
        radiances.irradiance = vec3(0.0);
        radiances.radiance = vec3(0.0);
        return radiances;
    }

    // Split-sum approximation for image based lighting: https://cdn2.unrealengine.com/Resources/files/2013SiggraphPresentationsNotes-26915738.pdf
    let radiance_level = perceptual_roughness * f32(textureNumLevels(bindings::specular_environment_maps[cubemap_index]) - 1u);

#ifndef LIGHTMAP
    radiances.irradiance = textureSampleLevel(
        bindings::diffuse_environment_maps[cubemap_index],
        bindings::environment_map_sampler,
        vec3(N.xy, -N.z),
        0.0).rgb;
#endif  // LIGHTMAP

    radiances.radiance = textureSampleLevel(
        bindings::specular_environment_maps[cubemap_index],
        bindings::environment_map_sampler,
        vec3(R.xy, -R.z),
        radiance_level).rgb;

    return radiances;
}

#else   // MULTIPLE_LIGHT_PROBES_IN_ARRAY

fn compute_radiances(
    perceptual_roughness: f32,
    N: vec3<f32>,
    R: vec3<f32>,
    world_position: vec3<f32>,
) -> EnvironmentMapRadiances {
    var radiances: EnvironmentMapRadiances;

    if (light_probes.view_cubemap_index < 0) {
        radiances.irradiance = vec3(0.0);
        radiances.radiance = vec3(0.0);
        return radiances;
    }

    // Split-sum approximation for image based lighting: https://cdn2.unrealengine.com/Resources/files/2013SiggraphPresentationsNotes-26915738.pdf
    // Technically we could use textureNumLevels(specular_environment_map) - 1 here, but we use a uniform
    // because textureNumLevels() does not work on WebGL2
    let radiance_level = perceptual_roughness * f32(light_probes.smallest_specular_mip_level_for_view);

#ifndef LIGHTMAP
    radiances.irradiance = textureSampleLevel(
        bindings::diffuse_environment_map,
        bindings::environment_map_sampler,
        vec3(N.xy, -N.z),
        0.0).rgb;
#endif  // LIGHTMAP

    radiances.radiance = textureSampleLevel(
        bindings::specular_environment_map,
        bindings::environment_map_sampler,
        vec3(R.xy, -R.z),
        radiance_level).rgb;

    return radiances;
}

#endif  // MULTIPLE_LIGHT_PROBES_IN_ARRAY

fn environment_map_light(
    perceptual_roughness: f32,
    roughness: f32,
    diffuse_color: vec3<f32>,
    NdotV: f32,
    f_ab: vec2<f32>,
    N: vec3<f32>,
    R: vec3<f32>,
    F0: vec3<f32>,
    world_position: vec3<f32>,
) -> EnvironmentMapLight {
    let radiances = compute_radiances(perceptual_roughness, N, R, world_position);

    // No real world material has specular values under 0.02, so we use this range as a
    // "pre-baked specular occlusion" that extinguishes the fresnel term, for artistic control.
    // See: https://google.github.io/filament/Filament.html#specularocclusion
    let specular_occlusion = saturate(dot(F0, vec3(50.0 * 0.33)));

    // Multiscattering approximation: https://www.jcgt.org/published/0008/01/03/paper.pdf
    // Useful reference: https://bruop.github.io/ibl
    let Fr = max(vec3(1.0 - roughness), F0) - F0;
    let kS = F0 + Fr * pow(1.0 - NdotV, 5.0);
    let Ess = f_ab.x + f_ab.y;
    let FssEss = kS * Ess * specular_occlusion;
    let Ems = 1.0 - Ess;
    let Favg = F0 + (1.0 - F0) / 21.0;
    let Fms = FssEss * Favg / (1.0 - Ems * Favg);
    let FmsEms = Fms * Ems;
    let Edss = 1.0 - (FssEss + FmsEms);
    let kD = diffuse_color * Edss;

    var out: EnvironmentMapLight;

    // If there's a lightmap, ignore the diffuse component of the reflection
    // probe, so we don't double-count light.
#ifdef LIGHTMAP
    out.diffuse = vec3(0.0);
#else
    out.diffuse = (FmsEms + kD) * radiances.irradiance;
#endif

    out.specular = FssEss * radiances.radiance;
    return out;
}
