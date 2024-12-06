#define_import_path bevy_pbr::mesh_functions

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

fn mesh_normal_local_to_world(vertex_normal: vec3<f32>) -> vec3<f32> {
    return mat3x3<f32>(
        mesh.inverse_transpose_model[0].xyz,
        mesh.inverse_transpose_model[1].xyz,
        mesh.inverse_transpose_model[2].xyz
    ) * vertex_normal;
}

fn mesh_tangent_local_to_world(model: mat4x4<f32>, vertex_tangent: vec4<f32>) -> vec4<f32> {
    return vec4<f32>(
        mat3x3<f32>(
            model[0].xyz,
            model[1].xyz,
            model[2].xyz
        ) * vertex_tangent.xyz,
        vertex_tangent.w
    );
}
