#define_import_path bevy_pbr::irradiance_volume

#import bevy_pbr::light_probe::query_light_probe
#import bevy_pbr::mesh_view_bindings::{
    irradiance_volumes,
    irradiance_volume,
    irradiance_volume_sampler,
    light_probes,
};

struct IrradianceVolumeLight {
    diffuse: vec3<f32>,
    found: bool,
}

// See:
// https://advances.realtimerendering.com/s2006/Mitchell-ShadingInValvesSourceEngine.pdf
// Slide 28, "Ambient Cube Basis"
fn sample_irradiance_volume(world_position: vec3<f32>, N: vec3<f32>) -> IrradianceVolumeLight {
    var irradiance_volume_light: IrradianceVolumeLight;

    // Search for an irradiance volume that contains the fragment.
    let query_result = query_light_probe(
        light_probes.irradiance_volumes,
        light_probes.irradiance_volume_count,
        world_position);

    // If there was no irradiance volume found, bail out.
    if (query_result.texture_index < 0) {
        irradiance_volume_light.found = false;
        return irradiance_volume_light;
    }

#ifdef MULTIPLE_LIGHT_PROBES_IN_ARRAY
    let volume_texture = irradiance_volumes[query_result.texture_index];
#else
    let volume_texture = irradiance_volume;
#endif

    let atlas_resolution = vec3<f32>(textureDimensions(volume_texture));
    let resolution = vec3<f32>(textureDimensions(volume_texture) / vec3(1u, 2u, 2u));

    // Make sure to clamp to the edges to avoid texture bleed.
    var unit_pos = (query_result.inverse_transform * vec4(world_position, 1.0f)).xyz;
    unit_pos = mix(vec3(-0.5f), vec3(resolution) + 0.5f, unit_pos + 0.5);

    let stp = clamp(unit_pos, vec3(0.5f), resolution - vec3(0.5f));
    let uvw = stp / atlas_resolution;

    let uvw_sh0   = uvw + vec3(0.0f, 0.0f, 0.0f);
    let uvw_sh1_x = uvw + vec3(0.0f, 0.0f, 0.5f);
    let uvw_sh1_y = uvw + vec3(0.0f, 0.5f, 0.0f);
    let uvw_sh1_z = uvw + vec3(0.0f, 0.5f, 0.5f);

    let rgb_sh0   = textureSample(volume_texture, irradiance_volume_sampler, uvw_sh0).rgb;
    let rgb_sh1_x = textureSample(volume_texture, irradiance_volume_sampler, uvw_sh1_x).rgb;
    let rgb_sh1_y = textureSample(volume_texture, irradiance_volume_sampler, uvw_sh1_y).rgb;
    let rgb_sh1_z = textureSample(volume_texture, irradiance_volume_sampler, uvw_sh1_z).rgb;

    // Reconstruct the irradiance distribution from the spherical harmonics.
    let NN = N * N;
    let r1_dot_n = rgb_sh1_x * N.x + rgb_sh1_y * N.y + rgb_sh1_z * N.z;
    irradiance_volume_light.diffuse = (rgb_sh0 + 2.0 * r1_dot_n) * query_result.intensity;
    irradiance_volume_light.found = true;
    return irradiance_volume_light;
}
