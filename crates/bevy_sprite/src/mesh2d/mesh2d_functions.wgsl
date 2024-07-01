#define_import_path bevy_sprite::mesh2d_functions

#import bevy_sprite::{
    mesh2d_view_bindings::view,
    mesh2d_bindings::mesh,
}
#import bevy_render::maths::{affine3_to_square, mat2x4_f32_to_mat3x3_unpack}

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

fn mesh2d_normal_local_to_world(vertex_normal: vec3<f32>, instance_index: u32) -> vec3<f32> {
    return mat2x4_f32_to_mat3x3_unpack(
        mesh[instance_index].local_from_world_transpose_a,
        mesh[instance_index].local_from_world_transpose_b,
    ) * vertex_normal;
}

fn mesh2d_tangent_local_to_world(world_from_local: mat4x4<f32>, vertex_tangent: vec4<f32>) -> vec4<f32> {
    return vec4<f32>(
        mat3x3<f32>(
            world_from_local[0].xyz,
            world_from_local[1].xyz,
            world_from_local[2].xyz
        ) * vertex_tangent.xyz,
        vertex_tangent.w
    );
}
