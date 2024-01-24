#define_import_path bevy_pbr::irradiance_volume

#import bevy_pbr::mesh_view_bindings::{
    irradiance_volumes,
    irradiance_volume,
    irradiance_volume_sampler,
    light_probes,
};
#import bevy_pbr::utils::transpose_affine_matrix

// See:
// https://advances.realtimerendering.com/s2006/Mitchell-ShadingInValvesSourceEngine.pdf
// Slide 28, "Ambient Cube Basis"
fn sample_irradiance_volume(P: vec3<f32>, N: vec3<f32>) -> vec3<f32> {
    // FIXME(pcwalton): Actually look up the proper light probe, once #10057 lands.
    let inverse_transform =
        transpose_affine_matrix(light_probes.irradiance_volumes[0].inverse_transpose_transform);

    let irradiance_volume_texture = irradiance_volumes[0];

    let atlas_resolution = vec3<f32>(textureDimensions(irradiance_volume_texture));
    let resolution = vec3<f32>(textureDimensions(irradiance_volume_texture) / vec3(1u, 2u, 3u));

    // Make sure to clamp to the edges to avoid texture bleed.
    let stp = clamp((inverse_transform * vec4(P, 1.0)).xyz, vec3(0.5f), resolution - vec3(0.5f));
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
    return rgb_x * NN.x + rgb_y * NN.y + rgb_z * NN.z;
}
