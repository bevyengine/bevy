fn mesh_model_position_to_world(
    model: mat4x4<f32>,
    vertex_position: vec4<f32>
) -> vec4<f32> {
    return model * vertex_position;
}

fn mesh_world_position_to_clip(
    view_proj: mat4x4<f32>,
    world_position: vec4<f32>
) -> vec4<f32> {
    return view_proj * world_position;
}

// NOTE: The intermediate world_position assignment is important
// for precision purposes when using the 'equals' depth comparison
// function.
fn mesh_model_position_to_clip(
    model: mat4x4<f32>,
    view_proj: mat4x4<f32>,
    vertex_position: vec4<f32>
) -> vec4<f32> {
    let world_position = mesh_model_position_to_world(model, vertex_position);
    return mesh_world_position_to_clip(view_proj, world_position);
}

fn mesh_model_normal_to_world(
    inverse_tranpose_model_3x3: mat3x3<f32>,
    vertex_normal: vec3<f32>
) -> vec3<f32> {
    return inverse_tranpose_model_3x3 * vertex_normal;
}

fn mesh_model_tangent_to_world(
    model: mat4x4<f32>,
    vertex_tangent: vec4<f32>
) -> vec4<f32> {
    return vec4<f32>(
        mat3x3<f32>(
            model[0].xyz,
            model[1].xyz,
            model[2].xyz
        ) * vertex_tangent.xyz,
        vertex_tangent.w
    );
}
