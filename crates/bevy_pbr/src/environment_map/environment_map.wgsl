#define_import_path bevy_pbr::environment_map

#import bevy_pbr::mesh_view_bindings as bindings

#import bevy_pbr::mesh_view_bindings       light_probes,

struct EnvironmentMapLight {
    diffuse: vec3<f32>,
    specular: vec3<f32>,
};

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
    // Search for a reflection probe that contains the fragment.
    //
    // TODO(pcwalton): Interpolate between multiple reflection probes.
    var cubemap_index: i32 = -1;
    for (var reflection_probe_index: i32 = 0;
            reflection_probe_index < light_probes.reflection_probe_count;
            reflection_probe_index += 1) {
        let reflection_probe = light_probes.reflection_probes[reflection_probe_index];
        let probe_space_pos =
            (reflection_probe.inverse_transform * vec4<f32>(world_position, 1.0)).xyz;
        if (all(abs(probe_space_pos) <= reflection_probe.half_extents)) {
            cubemap_index = reflection_probe.cubemap_index;
            break;
        }
    }

    // If we didn't find a reflection probe, use the view environment map if applicable.
    if (cubemap_index < 0) {
        cubemap_index = light_probes.view_cubemap_index;
    }

    var out: EnvironmentMapLight;
    // Split-sum approximation for image based lighting: https://cdn2.unrealengine.com/Resources/files/2013SiggraphPresentationsNotes-26915738.pdf
    // Technically we could use textureNumLevels(environment_map_specular) - 1 here, but we use a uniform
    // because textureNumLevels() does not work on WebGL2
    let radiance_level = perceptual_roughness * f32(bindings::lights.environment_map_smallest_specular_mip_level);

    // We always sample the first cubemap to achieve the required control flow uniformity (because of mip levels).
    let irradiance = textureSample(bindings::environment_map_diffuse, bindings::environment_map_sampler, vec3(N.xy, -N.z), max(cubemap_index, 0)).rgb;

    // If there's no cubemap, bail out.
    if (cubemap_index < 0) {
        out.diffuse = vec3(0.0);
        out.specular = vec3(0.0);
        return out;
    }

    let radiance = textureSampleLevel(bindings::environment_map_specular, bindings::environment_map_sampler, vec3(R.xy, -R.z), cubemap_index, radiance_level).rgb;

    // Multiscattering approximation: https://www.jcgt.org/published/0008/01/03/paper.pdf
    // Useful reference: https://bruop.github.io/ibl
    let Fr = max(vec3(1.0 - roughness), F0) - F0;
    let kS = F0 + Fr * pow(1.0 - NdotV, 5.0);
    let FssEss = kS * f_ab.x + f_ab.y;
    let Ess = f_ab.x + f_ab.y;
    let Ems = 1.0 - Ess;
    let Favg = F0 + (1.0 - F0) / 21.0;
    let Fms = FssEss * Favg / (1.0 - Ems * Favg);
    let FmsEms = Fms * Ems;
    let Edss = 1.0 - (FssEss + FmsEms);
    let kD = diffuse_color * Edss;

    out.diffuse = (FmsEms + kD) * irradiance;
    out.specular = FssEss * radiance;
    return out;
}
