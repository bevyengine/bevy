fn mesh_model_position_to_world(vertex_position: vec4<f32>) -> vec4<f32> {
    return mesh.model * vertex_position;
}

fn mesh_world_position_to_clip(world_position: vec4<f32>) -> vec4<f32> {
    return view.view_proj * world_position;
}

// NOTE: The intermediate world_position assignment is important
// for precision purposes when using the 'equals' depth comparison
// function.
fn mesh_model_position_to_clip(vertex_position: vec4<f32>) -> vec4<f32> {
    let world_position = mesh_model_position_to_world(vertex_position);
    return mesh_world_position_to_clip(world_position);
}

fn mesh_model_normal_to_world(vertex_normal: vec3<f32>) -> vec3<f32> {
    return mat3x3<f32>(
        mesh.inverse_transpose_model[0].xyz,
        mesh.inverse_transpose_model[1].xyz,
        mesh.inverse_transpose_model[2].xyz
    ) * vertex_normal;
}

fn mesh_model_tangent_to_world(vertex_tangent: vec4<f32>) -> vec4<f32> {
    return vec4<f32>(
        mat3x3<f32>(
            mesh.model[0].xyz,
            mesh.model[1].xyz,
            mesh.model[2].xyz
        ) * vertex_tangent.xyz,
        vertex_tangent.w
    );
}
