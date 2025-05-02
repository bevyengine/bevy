#import bevy_render::maths::{PI}
#import bevy_pbr::atmosphere::{
    types::{Atmosphere, AtmosphereSettings},
    bindings::{atmosphere, settings, probe_transform_buffer},
    functions::{sample_sky_view_lut, direction_world_to_atmosphere, max_atmosphere_distance, raymarch_atmosphere}
}

@group(0) @binding(13) var output: texture_storage_2d_array<rgba16float, write>;

// Convert from cubemap face and UV to direction vector
fn face_uv_to_direction(face: u32, uv: vec2<f32>) -> vec3<f32> {
    // Convert UV from [0,1] to [-1,1]
    let coords = 2.0 * uv - 1.0;
    
    // Generate direction based on face
    var dir: vec3<f32>;
    switch face {
        case 0u: { // +X
            dir = vec3<f32>(1.0, -coords.y, coords.x);
        }
        case 1u: { // -X
            dir = vec3<f32>(-1.0, -coords.y, -coords.x);
        }
        case 2u: { // +Y
            dir = vec3<f32>(coords.x, 1.0, -coords.y);
        }
        case 3u: { // -Y
            dir = vec3<f32>(coords.x, -1.0, coords.y);
        }
        case 4u: { // +Z
            dir = vec3<f32>(coords.x, -coords.y, -1.0);
        }
        default: { // -Z (5)
            dir = vec3<f32>(-coords.x, -coords.y, 1.0);
        }
    }
    
    return normalize(dir);
}

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

    var world_pos = probe_transform_buffer[3].xyz;

    // offset by the origin point of the atmosphere scene
    world_pos += vec3<f32>(0.0, atmosphere.bottom_radius, 0.0);

    let r = length(world_pos);

    let ray_dir_ws = face_uv_to_direction(slice_index, uv);
    // let ray_dir_as = direction_world_to_atmosphere(ray_dir_ws);
    // let inscattering = sample_sky_view_lut(r, ray_dir_as);
    let up = normalize(world_pos);
    let mu = dot(ray_dir_ws, up);
    let raymarch_steps = 16.0;
    let t_max = max_atmosphere_distance(r, mu);
    let sample_count = mix(1.0, raymarch_steps, clamp(t_max * 0.01, 0.0, 1.0));
    let result = raymarch_atmosphere(world_pos, ray_dir_ws, t_max, sample_count, uv, false, true, false);
    let inscattering = result.inscattering;
    let color = vec4<f32>(inscattering, 1.0);
    // let color = vec4<f32>(0.5, 0.5, 0.5, 1.0);

    // Write to the correct slice of the output texture
    textureStore(output, vec2<i32>(global_id.xy), i32(slice_index), color);
}
