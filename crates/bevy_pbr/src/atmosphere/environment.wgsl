#import bevy_pbr::{
    atmosphere::{
        functions::{direction_world_to_atmosphere, sample_sky_view_lut, get_view_position},
    },
    utils::sample_cube_dir
}

@group(0) @binding(13) var output: texture_storage_2d_array<rgba16float, write>;

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let dimensions = textureDimensions(output);
    let slice_index = global_id.z;
    
    if (global_id.x >= dimensions.x || global_id.y >= dimensions.y || slice_index >= 6u) {
        return;
    }
    
    // Calculate normalized UV coordinates for this pixel
    let uv = vec2<f32>(
        (f32(global_id.x) + 0.5) / f32(dimensions.x),
        (f32(global_id.y) + 0.5) / f32(dimensions.y)
    );

    var ray_dir_ws = sample_cube_dir(uv, slice_index);
    
    // invert the z direction to account for cubemaps being lefthanded
    ray_dir_ws.z = -ray_dir_ws.z;

    let world_pos = get_view_position();
    let r = length(world_pos);
    let up = normalize(world_pos);

    let ray_dir_as = direction_world_to_atmosphere(ray_dir_ws.xyz, up);
    let inscattering = sample_sky_view_lut(r, ray_dir_as);
    let color = vec4<f32>(inscattering, 1.0);

    textureStore(output, vec2<i32>(global_id.xy), i32(slice_index), color);
}