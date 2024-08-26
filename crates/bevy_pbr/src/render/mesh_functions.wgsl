#define_import_path bevy_pbr::mesh_functions

#import bevy_pbr::{
    mesh_view_bindings::{
        view,
        visibility_ranges,
        VISIBILITY_RANGE_UNIFORM_BUFFER_SIZE
    },
    mesh_bindings::mesh,
    mesh_types::MESH_FLAGS_SIGN_DETERMINANT_MODEL_3X3_BIT,
    view_transformations::position_world_to_clip,
}
#import bevy_render::maths::{affine3_to_square, mat2x4_f32_to_mat3x3_unpack}


fn get_world_from_local(instance_index: u32) -> mat4x4<f32> {
    return affine3_to_square(mesh[instance_index].world_from_local);
}

fn get_previous_world_from_local(instance_index: u32) -> mat4x4<f32> {
    return affine3_to_square(mesh[instance_index].previous_world_from_local);
}

fn mesh_position_local_to_world(world_from_local: mat4x4<f32>, vertex_position: vec4<f32>) -> vec4<f32> {
    return world_from_local * vertex_position;
}

// NOTE: The intermediate world_position assignment is important
// for precision purposes when using the 'equals' depth comparison
// function.
fn mesh_position_local_to_clip(world_from_local: mat4x4<f32>, vertex_position: vec4<f32>) -> vec4<f32> {
    let world_position = mesh_position_local_to_world(world_from_local, vertex_position);
    return position_world_to_clip(world_position.xyz);
}

fn mesh_normal_local_to_world(vertex_normal: vec3<f32>, instance_index: u32) -> vec3<f32> {
    // NOTE: The mikktspace method of normal mapping requires that the world normal is
    // re-normalized in the vertex shader to match the way mikktspace bakes vertex tangents
    // and normal maps so that the exact inverse process is applied when shading. Blender, Unity,
    // Unreal Engine, Godot, and more all use the mikktspace method.
    // We only skip normalization for invalid normals so that they don't become NaN.
    // Do not change this code unless you really know what you are doing.
    // http://www.mikktspace.com/
    if any(vertex_normal != vec3<f32>(0.0)) {
        return normalize(
            mat2x4_f32_to_mat3x3_unpack(
                mesh[instance_index].local_from_world_transpose_a,
                mesh[instance_index].local_from_world_transpose_b,
            ) * vertex_normal
        );
    } else {
        return vertex_normal;
    }
}

// Calculates the sign of the determinant of the 3x3 model matrix based on a
// mesh flag
fn sign_determinant_model_3x3m(mesh_flags: u32) -> f32 {
    // bool(u32) is false if 0u else true
    // f32(bool) is 1.0 if true else 0.0
    // * 2.0 - 1.0 remaps 0.0 or 1.0 to -1.0 or 1.0 respectively
    return f32(bool(mesh_flags & MESH_FLAGS_SIGN_DETERMINANT_MODEL_3X3_BIT)) * 2.0 - 1.0;
}

fn mesh_tangent_local_to_world(world_from_local: mat4x4<f32>, vertex_tangent: vec4<f32>, instance_index: u32) -> vec4<f32> {
    // NOTE: The mikktspace method of normal mapping requires that the world tangent is
    // re-normalized in the vertex shader to match the way mikktspace bakes vertex tangents
    // and normal maps so that the exact inverse process is applied when shading. Blender, Unity,
    // Unreal Engine, Godot, and more all use the mikktspace method.
    // We only skip normalization for invalid tangents so that they don't become NaN.
    // Do not change this code unless you really know what you are doing.
    // http://www.mikktspace.com/
    if any(vertex_tangent != vec4<f32>(0.0)) {
        return vec4<f32>(
            normalize(
                mat3x3<f32>(
                    world_from_local[0].xyz,
                    world_from_local[1].xyz,
                    world_from_local[2].xyz,
                ) * vertex_tangent.xyz
            ),
            // NOTE: Multiplying by the sign of the determinant of the 3x3 model matrix accounts for
            // situations such as negative scaling.
            vertex_tangent.w * sign_determinant_model_3x3m(mesh[instance_index].flags)
        );
    } else {
        return vertex_tangent;
    }
}

// Returns an appropriate dither level for the current mesh instance.
//
// This looks up the LOD range in the `visibility_ranges` table and compares the
// camera distance to determine the dithering level.
#ifdef VISIBILITY_RANGE_DITHER
fn get_visibility_range_dither_level(instance_index: u32, world_position: vec4<f32>) -> i32 {
#if AVAILABLE_STORAGE_BUFFER_BINDINGS >= 6
    // If we're using a storage buffer, then the length is variable.
    let visibility_buffer_array_len = arrayLength(&visibility_ranges);
#else   // AVAILABLE_STORAGE_BUFFER_BINDINGS >= 6
    // If we're using a uniform buffer, then the length is constant
    let visibility_buffer_array_len = VISIBILITY_RANGE_UNIFORM_BUFFER_SIZE;
#endif  // AVAILABLE_STORAGE_BUFFER_BINDINGS >= 6

    let visibility_buffer_index = mesh[instance_index].flags & 0xffffu;
    if (visibility_buffer_index > visibility_buffer_array_len) {
        return -16;
    }

    let lod_range = visibility_ranges[visibility_buffer_index];
    let camera_distance = length(view.world_position.xyz - world_position.xyz);

    // This encodes the following mapping:
    //
    //     `lod_range.`          x        y        z        w           camera distance
    //                   ←───────┼────────┼────────┼────────┼────────→
    //        LOD level  -16    -16       0        0        16      16  LOD level
    let offset = select(-16, 0, camera_distance >= lod_range.z);
    let bounds = select(lod_range.xy, lod_range.zw, camera_distance >= lod_range.z);
    let level = i32(round((camera_distance - bounds.x) / (bounds.y - bounds.x) * 16.0));
    return offset + clamp(level, 0, 16);
}
#endif
