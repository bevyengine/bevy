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
    let irradiance_volume_texture = irradiance_volumes[query_result.texture_index];
#else
    let irradiance_volume_texture = irradiance_volume;
#endif

    let atlas_resolution = vec3<f32>(textureDimensions(irradiance_volume_texture));
    let resolution = vec3<f32>(textureDimensions(irradiance_volume_texture) / vec3(1u, 2u, 3u));

    // Make sure to clamp to the edges to avoid texture bleed.
    var unit_pos = (query_result.inverse_transform * vec4(world_position, 1.0f)).xyz;
    unit_pos = mix(vec3(-0.5f), vec3(resolution) + 0.5f, unit_pos + 0.5);

    let stp = clamp(unit_pos, vec3(0.5f), resolution - vec3(0.5f));
    let uvw = stp / atlas_resolution;

    // The bottom half of each cube slice is the negative part, so choose it if applicable on each
    // slice.
    let neg_offset = select(vec3(0.0f), vec3(0.5f), N < vec3(0.0f));

    let uvw_x = uvw + vec3(0.0f, neg_offset.x, 0.0f);
    let uvw_y = uvw + vec3(0.0f, neg_offset.y, 1.0f / 3.0f);
    let uvw_z = uvw + vec3(0.0f, neg_offset.z, 2.0f / 3.0f);

    let rgb_x = textureSample(irradiance_volume_texture, irradiance_volume_sampler, uvw_x).rgb;
    let rgb_y = textureSample(irradiance_volume_texture, irradiance_volume_sampler, uvw_y).rgb;
    let rgb_z = textureSample(irradiance_volume_texture, irradiance_volume_sampler, uvw_z).rgb;

    // Use Valve's formula to sample.
    let NN = N * N;
    irradiance_volume_light.diffuse = (rgb_x * NN.x + rgb_y * NN.y + rgb_z * NN.z) *
        query_result.intensity;
    irradiance_volume_light.found = true;
    return irradiance_volume_light;
}
