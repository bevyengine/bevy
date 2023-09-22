#define_import_path bevy_pbr::mesh_functions

#import bevy_pbr::mesh_view_bindings  view
#import bevy_pbr::mesh_bindings       mesh
#import bevy_pbr::mesh_types          MESH_FLAGS_SIGN_DETERMINANT_MODEL_3X3_BIT
#import bevy_render::instance_index   get_instance_index
#import bevy_render::maths            affine_to_square, mat2x4_f32_to_mat3x3_unpack

fn get_model_matrix(instance_index: u32) -> mat4x4<f32> {
    return affine_to_square(mesh[get_instance_index(instance_index)].model);
}

fn get_previous_model_matrix(instance_index: u32) -> mat4x4<f32> {
    return affine_to_square(mesh[get_instance_index(instance_index)].previous_model);
}

fn mesh_position_local_to_world(model: mat4x4<f32>, vertex_position: vec4<f32>) -> vec4<f32> {
    return model * vertex_position;
}

fn mesh_position_world_to_clip(world_position: vec4<f32>) -> vec4<f32> {
    return view.view_proj * world_position;
}

// NOTE: The intermediate world_position assignment is important
// for precision purposes when using the 'equals' depth comparison
// function.
fn mesh_position_local_to_clip(model: mat4x4<f32>, vertex_position: vec4<f32>) -> vec4<f32> {
    let world_position = mesh_position_local_to_world(model, vertex_position);
    return mesh_position_world_to_clip(world_position);
}

fn mesh_normal_local_to_world(vertex_normal: vec3<f32>, instance_index: u32) -> vec3<f32> {
    // NOTE: The mikktspace method of normal mapping requires that the world normal is
    // re-normalized in the vertex shader to match the way mikktspace bakes vertex tangents
    // and normal maps so that the exact inverse process is applied when shading. Blender, Unity,
    // Unreal Engine, Godot, and more all use the mikktspace method. Do not change this code
    // unless you really know what you are doing.
    // http://www.mikktspace.com/
    return normalize(
        mat2x4_f32_to_mat3x3_unpack(
            mesh[instance_index].inverse_transpose_model_a,
            mesh[instance_index].inverse_transpose_model_b,
        ) * vertex_normal
    );
}

// Calculates the sign of the determinant of the 3x3 model matrix based on a
// mesh flag
fn sign_determinant_model_3x3m(instance_index: u32) -> f32 {
    // bool(u32) is false if 0u else true
    // f32(bool) is 1.0 if true else 0.0
    // * 2.0 - 1.0 remaps 0.0 or 1.0 to -1.0 or 1.0 respectively
    return f32(bool(mesh[instance_index].flags & MESH_FLAGS_SIGN_DETERMINANT_MODEL_3X3_BIT)) * 2.0 - 1.0;
}

fn mesh_tangent_local_to_world(model: mat4x4<f32>, vertex_tangent: vec4<f32>, instance_index: u32) -> vec4<f32> {
    // NOTE: The mikktspace method of normal mapping requires that the world tangent is
    // re-normalized in the vertex shader to match the way mikktspace bakes vertex tangents
    // and normal maps so that the exact inverse process is applied when shading. Blender, Unity,
    // Unreal Engine, Godot, and more all use the mikktspace method. Do not change this code
    // unless you really know what you are doing.
    // http://www.mikktspace.com/
    return vec4<f32>(
        normalize(
            mat3x3<f32>(
                model[0].xyz,
                model[1].xyz,
                model[2].xyz
            ) * vertex_tangent.xyz
        ),
        // NOTE: Multiplying by the sign of the determinant of the 3x3 model matrix accounts for
        // situations such as negative scaling.
        vertex_tangent.w * sign_determinant_model_3x3m(instance_index)
    );
}
