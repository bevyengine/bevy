#define_import_path bevy_pbr::irradiance_volumes

#import bevy_pbr::mesh_view_bindings::{
    irradiance_volumes,
    irradiance_volume_data
};

fn load_irradiance_cell(offset: u32, N: vec3<f32>) -> mat3x3<f32> {
    let is_negative = vec3<u32>(N < vec3(0.0));

    return mat3x3<f32>(
        irradiance_volume_data.data[offset + 0u + is_negative.x].rgb,
        irradiance_volume_data.data[offset + 2u + is_negative.y].rgb,
        irradiance_volume_data.data[offset + 4u + is_negative.z].rgb);
}

// Returns the voxel color, premultiplied by weight, in the `rgb` field and the weight in `a`.
fn accumulate_voxel_irradiance(
        voxel_origin: vec3<f32>,
        voxel_offset: vec3<i32>,
        voxel_fract: vec3<f32>,
        resolution: vec3<u32>,
        transform: mat4x4<f32>,
        start_offset: u32,
        P: vec3<f32>,
        N: vec3<f32>)
        -> vec4<f32> {
    let voxel_coords = clamp(
        voxel_origin + vec3<f32>(voxel_offset),
        vec3(0.0),
        vec3<f32>(resolution) - 1.0);

    let voxel_coords_u = vec3<u32>(voxel_coords);
    let cell_index = voxel_coords_u.z +
        resolution.z * (voxel_coords_u.y + resolution.y * voxel_coords_u.x);

    let irradiance = load_irradiance_cell(start_offset + cell_index * 6u, N) * (N * N);

    // FIXME(pcwalton): I think we can avoid multiplying transform every time here, since the
    // offset is always -1, 0, or 1.
    var weight = clamp(
        dot(normalize((transform * vec4(voxel_coords, 1.0)).xyz - P), N), 0.0, 1.0) + 1.0;
    let interpolated_color = mix(1.0 - voxel_fract, voxel_fract, vec3<f32>(voxel_offset));

    weight = weight * interpolated_color.x * interpolated_color.y * interpolated_color.z;
    return vec4(irradiance * weight, weight);
}

fn sample_irradiance_volume(P: vec3<f32>, N: vec3<f32>) -> vec3<f32> {
    // FIXME(pcwalton): Actually look up the proper light probe, once #10057 lands.
    let resolution = vec3<u32>(irradiance_volumes.data[0].resolution);
    let transform = irradiance_volumes.data[0].transform;
    let inverse_transform = irradiance_volumes.data[0].inverse_transform;
    let start_offset = irradiance_volumes.data[0].start_offset;

    let voxel_coords = (inverse_transform * vec4(P, 1.0)).xyz;
    let voxel_origin = floor(voxel_coords);
    let voxel_fract = fract(voxel_coords);

    var total_irradiance_and_weight = vec4(0.0);
    for (var i = 0; i < 8; i++) {
        let voxel_offset = vec3(i, i / 2, i / 4) & vec3(1);
        total_irradiance_and_weight += accumulate_voxel_irradiance(
            voxel_origin,
            voxel_offset,
            voxel_fract,
            resolution,
            transform,
            start_offset,
            P,
            N
        );
    }

    // Avoid division by zero.
    if (total_irradiance_and_weight.a < 0.0001) {
        return vec3(0.0);
    }

    return total_irradiance_and_weight.rgb / total_irradiance_and_weight.a;
}
