#define_import_path bevy_pbr::environment_map

#import bevy_pbr::light_probe::query_light_probe
#import bevy_pbr::mesh_view_bindings as bindings
#import bevy_pbr::mesh_view_bindings::light_probes
#import bevy_pbr::lighting::{F_Schlick_vec, LightingInput}

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
    found_diffuse_indirect: bool,
) -> EnvironmentMapRadiances {
    var radiances: EnvironmentMapRadiances;

    // Search for a reflection probe that contains the fragment.
    var query_result = query_light_probe(world_position, /*is_irradiance_volume=*/ false);

    // If we didn't find a reflection probe, use the view environment map if applicable.
    if (query_result.texture_index < 0) {
        query_result.texture_index = light_probes.view_cubemap_index;
        query_result.intensity = light_probes.intensity_for_view;
    }

    // If there's no cubemap, bail out.
    if (query_result.texture_index < 0) {
        radiances.irradiance = vec3(0.0);
        radiances.radiance = vec3(0.0);
        return radiances;
    }

    // Split-sum approximation for image based lighting: https://cdn2.unrealengine.com/Resources/files/2013SiggraphPresentationsNotes-26915738.pdf
    let radiance_level = perceptual_roughness * f32(textureNumLevels(
        bindings::specular_environment_maps[query_result.texture_index]) - 1u);

    if (!found_diffuse_indirect) {
        radiances.irradiance = textureSampleLevel(
            bindings::diffuse_environment_maps[query_result.texture_index],
            bindings::environment_map_sampler,
            vec3(N.xy, -N.z),
            0.0).rgb * query_result.intensity;
    }

    radiances.radiance = textureSampleLevel(
        bindings::specular_environment_maps[query_result.texture_index],
        bindings::environment_map_sampler,
        vec3(R.xy, -R.z),
        radiance_level).rgb * query_result.intensity;

    return radiances;
}

#else   // MULTIPLE_LIGHT_PROBES_IN_ARRAY

fn compute_radiances(
    perceptual_roughness: f32,
    N: vec3<f32>,
    R: vec3<f32>,
    world_position: vec3<f32>,
    found_diffuse_indirect: bool,
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

    let intensity = light_probes.intensity_for_view;

    if (!found_diffuse_indirect) {
        radiances.irradiance = textureSampleLevel(
            bindings::diffuse_environment_map,
            bindings::environment_map_sampler,
            vec3(N.xy, -N.z),
            0.0).rgb * intensity;
    }

    radiances.radiance = textureSampleLevel(
        bindings::specular_environment_map,
        bindings::environment_map_sampler,
        vec3(R.xy, -R.z),
        radiance_level).rgb * intensity;

    return radiances;
}

#endif  // MULTIPLE_LIGHT_PROBES_IN_ARRAY

fn environment_map_light(
    input: ptr<function, LightingInput>,
    found_diffuse_indirect: bool,
) -> EnvironmentMapLight {
    // Unpack.
    let perceptual_roughness = (*input).perceptual_roughness;
    let roughness = (*input).roughness;
    let diffuse_color = (*input).diffuse_color;
    let NdotV = (*input).NdotV;
    let f_ab = (*input).f_ab;
    let N = (*input).N;
    let R = (*input).R;
    let F0 = (*input).Fo;
#ifdef STANDARD_MATERIAL_CLEARCOAT
    let clearcoat_NdotV = (*input).clearcoat_NdotV;
    let clearcoat_N = (*input).clearcoat_N;
    let clearcoat_R = (*input).clearcoat_R;
    let clearcoat = (*input).clearcoat;
    let clearcoat_perceptual_roughness = (*input).clearcoat_perceptual_roughness;
#endif
    let world_position = (*input).P;

    var out: EnvironmentMapLight;

    let radiances = compute_radiances(
        perceptual_roughness,
        N,
        R,
        world_position,
        found_diffuse_indirect);
    if (all(radiances.irradiance == vec3(0.0)) && all(radiances.radiance == vec3(0.0))) {
        out.diffuse = vec3(0.0);
        out.specular = vec3(0.0);
        return out;
    }

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

    if (!found_diffuse_indirect) {
        out.diffuse = (FmsEms + kD) * radiances.irradiance;
    } else {
        out.diffuse = vec3(0.0);
    }

    out.specular = FssEss * radiances.radiance;

    // Clearcoat

#ifdef STANDARD_MATERIAL_CLEARCOAT

    // Calculate the Fresnel term `Fc` for the clearcoat layer.
    // 0.04 is a hardcoded value for F0 from the Filament spec.
    let clearcoat_F0 = vec3<f32>(0.04);
    let Fc = F_Schlick_vec(clearcoat_F0, 1.0, clearcoat_NdotV) * clearcoat;
    let inv_Fc = 1.0 - Fc;
    let clearcoat_radiances = compute_radiances(
        clearcoat_perceptual_roughness,
        clearcoat_N,
        clearcoat_R,
        world_position,
        found_diffuse_indirect);

    // Composite the clearcoat layer on top of the existing one.
    // These formulas are from Filament:
    // <https://google.github.io/filament/Filament.md.html#lighting/imagebasedlights/clearcoat>
    out.diffuse *= inv_Fc;
    out.specular = out.specular * inv_Fc * inv_Fc + clearcoat_radiances.radiance * Fc;

#endif  // STANDARD_MATERIAL_CLEARCOAT

    return out;
}
