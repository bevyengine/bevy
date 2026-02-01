#define_import_path bevy_pbr::environment_map

#import bevy_pbr::light_probe::{light_probe_iterator_new, light_probe_iterator_next}
#import bevy_pbr::mesh_view_bindings as bindings
#import bevy_pbr::mesh_view_bindings::light_probes
#import bevy_pbr::mesh_view_bindings::environment_map_uniform
#import bevy_pbr::mesh_view_types::{
    LIGHT_PROBE_FLAG_AFFECTS_LIGHTMAPPED_MESH_DIFFUSE, LIGHT_PROBE_FLAG_PARALLAX_CORRECT
}
#import bevy_pbr::lighting::{F_Schlick_vec, LightingInput, LayerLightingInput, LAYER_BASE, LAYER_CLEARCOAT}
#import bevy_pbr::clustered_forward::ClusterableObjectIndexRanges

// The maximum representable value in a 32-bit floating point number.
const FLOAT_MAX: f32 = 3.40282347e+38;

struct EnvironmentMapLight {
    diffuse: vec3<f32>,
    specular: vec3<f32>,
};

struct EnvironmentMapRadiances {
    irradiance: vec3<f32>,
    radiance: vec3<f32>,
}

// Computes the direction at which to sample the reflection probe.
fn compute_cubemap_sample_dir(
    world_ray_origin: vec3<f32>,
    world_ray_direction: vec3<f32>,
    light_from_world: mat4x4<f32>,
    parallax_correct: bool
) -> vec3<f32> {
    var sample_dir: vec3<f32>;

    // If we're supposed to parallax correct, then intersect with the light cube.
    if (parallax_correct) {
        // Compute the direction of the ray bouncing off the surface, in light
        // probe space.
        // Recall that light probe space is a 1×1×1 cube centered at the origin.
        let ray_origin = (light_from_world * vec4(world_ray_origin, 1.0)).xyz;
        let ray_direction = (light_from_world * vec4(world_ray_direction, 0.0)).xyz;

        // Solve for the intersection of that ray with each side of the cube.
        // Since our light probe is a 1×1×1 cube centered at the origin in light
        // probe space, the faces of the cube are at X = ±0.5, Y = ±0.5, and Z =
        // ±0.5.
        var t0 = (vec3(-0.5) - ray_origin) / ray_direction;
        var t1 = (vec3(0.5) - ray_origin) / ray_direction;

        // We're shooting the rays forward, so we need to rule out negative time
        // values. So, if t is negative, make it a large value so that we won't
        // choose it below.
        // We would use infinity here but WGSL forbids it:
        // https://github.com/gfx-rs/wgpu/issues/5515
        t0 = select(vec3(FLOAT_MAX), t0, t0 >= vec3(0.0));
        t1 = select(vec3(FLOAT_MAX), t1, t1 >= vec3(0.0));

        // Choose the minimum valid time value to find the intersection of the
        // first cube face.
        let t_min = min(t0, t1);
        let t = min(min(t_min.x, t_min.y), t_min.z);

        // Compute the sample direction. (It doesn't have to be normalized.)
        sample_dir = ray_origin + ray_direction * t;
    } else {
        // We treat the reflection as infinitely far away in the non-parallax
        // case, so the ray origin is irrelevant.
        sample_dir = (light_from_world * vec4(world_ray_direction, 0.0)).xyz;
    }

    // Cubemaps are left-handed, so we negate the Z coordinate.
    sample_dir.z = -sample_dir.z;
    return sample_dir;
}

// Define two versions of this function, one for the case in which there are
// multiple light probes and one for the case in which only the view light probe
// is present.

#ifdef MULTIPLE_LIGHT_PROBES_IN_ARRAY

fn compute_radiances(
    input: LayerLightingInput,
    clusterable_object_index_ranges: ptr<function, ClusterableObjectIndexRanges>,
    world_position: vec3<f32>,
    found_diffuse_indirect: bool,
) -> EnvironmentMapRadiances {
    // Unpack.
    let N = input.N;
    let R = input.R;
    let perceptual_roughness = input.perceptual_roughness;
    let roughness = input.roughness;

    var radiances: EnvironmentMapRadiances;

    // Find all reflection probes that contain the fragment. We're going to
    // accumulate all the radiance and irradiance from them in a weighted sum.
    var iterator = light_probe_iterator_new(
        world_position,
        /*is_irradiance_volume=*/ false,
        clusterable_object_index_ranges,
    );

    var total_weight = 0.0;
    radiances.irradiance = vec3(0.0);
    radiances.radiance = vec3(0.0);

    while (true) {
        var query_result = light_probe_iterator_next(&iterator);

        // If we reached the end of the light probe list, and we didn't find
        // enough reflection probes to reach a weight of 1.0, use the view
        // environment map if applicable. This allows for e.g. nice transitions
        // between the interior of a building and the outdoor environment map.
        if (query_result.texture_index < 0 && total_weight < 0.9999) {
            query_result.texture_index = light_probes.view_cubemap_index;
            query_result.intensity = light_probes.intensity_for_view;
            query_result.light_from_world = mat4x4(
                vec4(1.0, 0.0, 0.0, 0.0),
                vec4(0.0, 1.0, 0.0, 0.0),
                vec4(0.0, 0.0, 1.0, 0.0),
                vec4(0.0, 0.0, 0.0, 1.0)
            );
            if light_probes.view_environment_map_affects_lightmapped_mesh_diffuse != 0u {
                query_result.flags = LIGHT_PROBE_FLAG_AFFECTS_LIGHTMAPPED_MESH_DIFFUSE;
            } else {
                query_result.flags = 0u;
            }
            query_result.weight = 1.0 - total_weight;
        }

        // If we reached the end, we're done.
        if (query_result.texture_index < 0) {
            break;
        }

        // Split-sum approximation for image based lighting: https://cdn2.unrealengine.com/Resources/files/2013SiggraphPresentationsNotes-26915738.pdf
        let radiance_level = perceptual_roughness * f32(textureNumLevels(
            bindings::specular_environment_maps[query_result.texture_index]) - 1u);

        // If we're lightmapped, and we shouldn't accumulate diffuse light from the
        // environment map, note that.
        var enable_diffuse = !found_diffuse_indirect;
#ifdef LIGHTMAP
        enable_diffuse = enable_diffuse &&
            (query_result.flags & LIGHT_PROBE_FLAG_AFFECTS_LIGHTMAPPED_MESH_DIFFUSE) != 0u;
#endif  // LIGHTMAP

        let parallax_correct = (query_result.flags & LIGHT_PROBE_FLAG_PARALLAX_CORRECT) != 0u;

        if (enable_diffuse) {
            let irradiance_sample_dir = compute_cubemap_sample_dir(
                world_position,
                N,
                query_result.light_from_world,
                parallax_correct
            );
            radiances.irradiance = textureSampleLevel(
                bindings::diffuse_environment_maps[query_result.texture_index],
                bindings::environment_map_sampler,
                irradiance_sample_dir,
                0.0).rgb * query_result.intensity * query_result.weight;
        }

        var radiance_sample_dir = radiance_sample_direction(N, R, roughness);
        radiance_sample_dir = compute_cubemap_sample_dir(
            world_position,
            radiance_sample_dir,
            query_result.light_from_world,
            parallax_correct
        );
        radiances.radiance +=
            textureSampleLevel(
                bindings::specular_environment_maps[query_result.texture_index],
                bindings::environment_map_sampler,
                radiance_sample_dir,
                radiance_level).rgb * query_result.intensity * query_result.weight;

        total_weight += query_result.weight;
    }

    if (total_weight != 0.0) {
        radiances.irradiance /= total_weight;
        radiances.radiance /= total_weight;
    }

    return radiances;
}

#else   // MULTIPLE_LIGHT_PROBES_IN_ARRAY

fn compute_radiances(
    input: LayerLightingInput,
    clusterable_object_index_ranges: ptr<function, ClusterableObjectIndexRanges>,
    world_position: vec3<f32>,
    found_diffuse_indirect: bool,
) -> EnvironmentMapRadiances {
    // Unpack.
    let N = input.N;
    let R = input.R;
    let perceptual_roughness = input.perceptual_roughness;
    let roughness = input.roughness;

    var radiances: EnvironmentMapRadiances;

    // If we have no light probe, bail.
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

    // If we're lightmapped, and we shouldn't accumulate diffuse light from the
    // environment map, note that.
    var enable_diffuse = !found_diffuse_indirect;
#ifdef LIGHTMAP
    enable_diffuse = enable_diffuse &&
        light_probes.view_environment_map_affects_lightmapped_mesh_diffuse;
#endif  // LIGHTMAP

    if (enable_diffuse) {
        var irradiance_sample_dir = N;
        // Rotating the world space ray direction by the environment light map transform matrix, it is
        // equivalent to rotating the diffuse environment cubemap itself.
        irradiance_sample_dir = (environment_map_uniform.transform * vec4(irradiance_sample_dir, 1.0)).xyz;
        // Cube maps are left-handed so we negate the z coordinate.
        irradiance_sample_dir.z = -irradiance_sample_dir.z;
        radiances.irradiance = textureSampleLevel(
            bindings::diffuse_environment_map,
            bindings::environment_map_sampler,
            irradiance_sample_dir,
            0.0).rgb * intensity;
    }

    var radiance_sample_dir = radiance_sample_direction(N, R, roughness);
    // Rotating the world space ray direction by the environment light map transform matrix, it is
    // equivalent to rotating the specular environment cubemap itself.
    radiance_sample_dir = (environment_map_uniform.transform * vec4(radiance_sample_dir, 1.0)).xyz;
    // Cube maps are left-handed so we negate the z coordinate.
    radiance_sample_dir.z = -radiance_sample_dir.z;
    radiances.radiance = textureSampleLevel(
        bindings::specular_environment_map,
        bindings::environment_map_sampler,
        radiance_sample_dir,
        radiance_level).rgb * intensity;

    return radiances;
}

#endif  // MULTIPLE_LIGHT_PROBES_IN_ARRAY

#ifdef STANDARD_MATERIAL_CLEARCOAT

// Adds the environment map light from the clearcoat layer to that of the base
// layer.
fn environment_map_light_clearcoat(
    out: ptr<function, EnvironmentMapLight>,
    input: ptr<function, LightingInput>,
    clusterable_object_index_ranges: ptr<function, ClusterableObjectIndexRanges>,
    found_diffuse_indirect: bool,
) {
    // Unpack.
    let world_position = (*input).P;
    let clearcoat_NdotV = (*input).layers[LAYER_CLEARCOAT].NdotV;
    let clearcoat_strength = (*input).clearcoat_strength;

    // Calculate the Fresnel term `Fc` for the clearcoat layer.
    // 0.04 is a hardcoded value for F0 from the Filament spec.
    let clearcoat_F0 = vec3<f32>(0.04);
    let Fc = F_Schlick_vec(clearcoat_F0, 1.0, clearcoat_NdotV) * clearcoat_strength;
    let inv_Fc = 1.0 - Fc;

    let clearcoat_radiances = compute_radiances(
        (*input).layers[LAYER_CLEARCOAT],
        clusterable_object_index_ranges,
        world_position,
        found_diffuse_indirect,
    );

    // Composite the clearcoat layer on top of the existing one.
    // These formulas are from Filament:
    // <https://google.github.io/filament/Filament.md.html#lighting/imagebasedlights/clearcoat>
    (*out).diffuse *= inv_Fc;
    (*out).specular = (*out).specular * inv_Fc * inv_Fc + clearcoat_radiances.radiance * Fc;
}

#endif  // STANDARD_MATERIAL_CLEARCOAT

fn environment_map_light(
    input: ptr<function, LightingInput>,
    clusterable_object_index_ranges: ptr<function, ClusterableObjectIndexRanges>,
    found_diffuse_indirect: bool,
) -> EnvironmentMapLight {
    // Unpack.
    let roughness = (*input).layers[LAYER_BASE].roughness;
    let diffuse_color = (*input).diffuse_color;
    let NdotV = (*input).layers[LAYER_BASE].NdotV;
    let F_ab = (*input).F_ab;
    let F0 = (*input).F0_;
    let world_position = (*input).P;

    var out: EnvironmentMapLight;

    let radiances = compute_radiances(
        (*input).layers[LAYER_BASE],
        clusterable_object_index_ranges,
        world_position,
        found_diffuse_indirect,
    );

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
    // We initially used this (https://bruop.github.io/ibl) reference with Roughness Dependent
    // Fresnel, but it made fresnel very bright so we reverted to the "typical" fresnel term.
    let FssEss = (F0 * F_ab.x + F_ab.y) * specular_occlusion;
    let Ems = 1.0 - (F_ab.x + F_ab.y);
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

#ifdef STANDARD_MATERIAL_CLEARCOAT
    environment_map_light_clearcoat(
        &out,
        input,
        clusterable_object_index_ranges,
        found_diffuse_indirect,
    );
#endif  // STANDARD_MATERIAL_CLEARCOAT

    return out;
}

// "Moving Frostbite to Physically Based Rendering 3.0", listing 22
// https://seblagarde.wordpress.com/wp-content/uploads/2015/07/course_notes_moving_frostbite_to_pbr_v32.pdf#page=70
fn radiance_sample_direction(N: vec3<f32>, R: vec3<f32>, roughness: f32) -> vec3<f32> {
    let smoothness = saturate(1.0 - roughness);
    let lerp_factor = smoothness * (sqrt(smoothness) + roughness);
    return mix(N, R, lerp_factor);
}
