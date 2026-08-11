import bevy_sprite_render::mesh2d::{
    view_bindings::view,
    bindings::{mesh, metadata},
    types::MESH_FLAGS_SIGN_DETERMINANT_MODEL_3X3_BIT,
};
import bevy_render::mesh::metadata_types::MeshMetadata;
import bevy_render::maths::{affine3_to_square, mat2x4_f32_to_mat3x3_unpack};

fn get_world_from_local(instance_index: u32) -> mat4x4<f32> {
    return affine3_to_square(mesh[instance_index].world_from_local);
}

fn mesh2d_position_local_to_world(world_from_local: mat4x4<f32>, vertex_position: vec4<f32>) -> vec4<f32> {
    return world_from_local * vertex_position;
}

fn mesh2d_position_world_to_clip(world_position: vec4<f32>) -> vec4<f32> {
    return view.clip_from_world * world_position;
}

// NOTE: The intermediate world_position assignment is important
// for precision purposes when using the 'equals' depth comparison
// function.
fn mesh2d_position_local_to_clip(world_from_local: mat4x4<f32>, vertex_position: vec4<f32>) -> vec4<f32> {
    let world_position = mesh2d_position_local_to_world(world_from_local, vertex_position);
    return mesh2d_position_world_to_clip(world_position);
}

// Matches the `mesh_normal_local_to_world` function in `bevy_pbr/mesh_functions.wesl`.
fn mesh2d_normal_local_to_world(vertex_normal: vec3<f32>, instance_index: u32) -> vec3<f32> {
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
// mesh flag.
// Matches the `sign_determinant_model_3x3m` function in `bevy_pbr/mesh_functions.wesl`.
fn sign_determinant_model_3x3m(mesh_flags: u32) -> f32 {
    // bool(u32) is false if 0u else true
    // f32(bool) is 1.0 if true else 0.0
    // * 2.0 - 1.0 remaps 0.0 or 1.0 to -1.0 or 1.0 respectively
    return f32(bool(mesh_flags & MESH_FLAGS_SIGN_DETERMINANT_MODEL_3X3_BIT)) * 2.0 - 1.0;
}

// Matches the `mesh_tangent_local_to_world` function in `bevy_pbr/mesh_functions.wesl`.
fn mesh2d_tangent_local_to_world(world_from_local: mat4x4<f32>, vertex_tangent: vec4<f32>, instance_index: u32) -> vec4<f32> {
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
                    world_from_local[2].xyz
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

fn get_tag(instance_index: u32) -> u32 {
    return mesh[instance_index].tag;
}

fn get_metadata(instance_index: u32) -> MeshMetadata {
@if(METADATA_USE_UNIFORM_BUFFERS)
    return metadata;
@else
    return metadata[mesh[instance_index].metadata_index];
}
