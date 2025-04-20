#import bevy_render::maths::PI_2
#import bevy_pbr::atmosphere::{
    types::{Atmosphere, AtmosphereSettings},
    bindings::{atmosphere, settings},
    functions::{PI, PI_1_2, sample_sky_view_lut, direction_world_to_atmosphere}
}

struct ComputeParams {
    slice_index: u32,
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
            dir = vec3<f32>(coords.x, 1.0, coords.y);
        }
        case 3u: { // -Y
            dir = vec3<f32>(coords.x, -1.0, -coords.y);
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


// Generate a test pattern for debugging
fn test_pattern(face: u32, uv: vec2<f32>) -> vec4<f32> {
    // Base colors for each face
    var base_color: vec3<f32>;
    switch face {
        case 0u: { // +X (Right face) - Red
            base_color = vec3<f32>(1.0, 0.2, 0.2);
        }
        case 1u: { // -X (Left face) - Green
            base_color = vec3<f32>(0.2, 1.0, 0.2);
        }
        case 2u: { // +Y (Top face) - Blue
            base_color = vec3<f32>(0.2, 0.2, 1.0);
        }
        case 3u: { // -Y (Bottom face) - Yellow
            base_color = vec3<f32>(1.0, 1.0, 0.2);
        }
        case 4u: { // +Z (Front face) - Cyan
            base_color = vec3<f32>(0.2, 1.0, 1.0);
        }
        default: { // -Z (Back face) - Magenta
            base_color = vec3<f32>(1.0, 0.2, 1.0);
        }
    }
    
    // Checkerboard pattern
    let checker_size = 8.0;
    let cx = floor(uv.x * checker_size);
    let cy = floor(uv.y * checker_size);
    // let checker = mod(cx + cy, 2.0);
    let checker = (cx + cy) % 2.0;
    
    // Direction markers (arrows pointing to center)
    let dist_x = abs(uv.x - 0.5) * 2.0;
    let dist_y = abs(uv.y - 0.5) * 2.0;
    let edge_highlight = step(0.8, max(dist_x, dist_y));
    
    // Face index as text in the center
    let center_marker = step(distance(uv, vec2<f32>(0.5, 0.5)), 0.1);
    
    // Combine patterns
    let pattern = mix(0.5, 1.0, checker);
    var color = base_color * pattern;
    
    // Add edge highlight and center marker
    color = mix(color, vec3<f32>(1.0), edge_highlight * 0.5);
    color = mix(color, vec3<f32>(1.0, 1.0, 1.0), center_marker);
    
    return vec4<f32>(color, 1.0);
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
    
    // OPTION 1: Use test pattern
    let colorDebug = test_pattern(slice_index, uv);

    let ray_dir_ws = face_uv_to_direction(slice_index, uv);
    let r = atmosphere.bottom_radius;

    let ray_dir_as = direction_world_to_atmosphere(ray_dir_ws);
    let inscattering = sample_sky_view_lut(r, ray_dir_as);
    let color = vec4<f32>(inscattering, 1.0);

    // Write to the correct slice of the output texture
    textureStore(output, vec2<i32>(global_id.xy), i32(slice_index), color);
} 

