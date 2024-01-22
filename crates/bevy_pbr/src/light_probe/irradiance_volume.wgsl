#define_import_path bevy_pbr::irradiance_volume

#import bevy_pbr::mesh_view_bindings::{
    irradiance_volumes,
    irradiance_volume_data
};

// See "Section 3.8.x, Shared Exponent Texture Color Conversion" in
// https://registry.khronos.org/OpenGL/extensions/EXT/EXT_texture_shared_exponent.txt
//
// But note that there is an error in that section: `2^(exp_shared - B)` should
// be `2^(exp_shared - B - N)`.
fn decode_rgb9e5(packed: u32) -> vec3<f32> {
    let B = 15;
    let N = 9;
    let expt = i32(packed >> 27u) - B - N;
    return vec3<f32>(
        ldexp(f32(packed & 0x1ffu), expt),
        ldexp(f32((packed >> 9u) & 0x1ffu), expt),
        ldexp(f32((packed >> 18u) & 0x1ffu), expt));
}

fn load_irradiance_cell(offset: u32, N: vec3<f32>) -> mat3x3<f32> {
    let is_negative = vec3<u32>(N < vec3(0.0));

    return mat3x3<f32>(
        decode_rgb9e5(irradiance_volume_data.data[offset + 0u + is_negative.x]),
        decode_rgb9e5(irradiance_volume_data.data[offset + 2u + is_negative.y]),
        decode_rgb9e5(irradiance_volume_data.data[offset + 4u + is_negative.z]));
}

// Returns the voxel color, premultiplied by weight, in the `rgb` field and the weight in `a`.
fn accumulate_voxel_irradiance(
        voxel_origin: vec3<f32>,
        voxel_offset: vec3<i32>,
        resolution: vec3<u32>,
        transform: mat4x4<f32>,
        start_offset: u32,
        P: vec3<f32>,
        N: vec3<f32>)
        -> vec3<f32> {
    let voxel_coords = clamp(
        voxel_origin + vec3<f32>(voxel_offset),
        vec3(0.0),
        vec3<f32>(resolution) - 1.0);

    let voxel_coords_u = vec3<u32>(voxel_coords);
    let cell_index = voxel_coords_u.z +
        resolution.z * (voxel_coords_u.y + resolution.y * voxel_coords_u.x);

    //let irradiance = load_irradiance_cell(start_offset + cell_index * 6u, N) * /*(N * N)*/N;
    //return load_irradiance_cell(start_offset + cell_index * 6u, N)[0];
    return load_irradiance_cell(start_offset + cell_index * 6u, N) * abs(N);
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

    var octet: array<vec3<f32>, 8>;
    for (var i = 0; i < 8; i++) {
        let voxel_offset = vec3(i, i / 2, i / 4) & vec3(1);
        octet[i] = accumulate_voxel_irradiance(
            voxel_origin,
            voxel_offset,
            resolution,
            transform,
            start_offset,
            P,
            N
        );
    }

    // Trilinearly filter.
    return mix(
        mix(
            mix(octet[0], octet[1], voxel_fract.x),
            mix(octet[2], octet[3], voxel_fract.x),
            voxel_fract.y),
        mix(
            mix(octet[4], octet[5], voxel_fract.x),
            mix(octet[6], octet[7], voxel_fract.x),
            voxel_fract.y),
        voxel_fract.z,
    );
}
