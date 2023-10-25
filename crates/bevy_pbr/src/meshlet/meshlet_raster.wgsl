#import bevy_pbr::{
    meshlet_bindings::{
        meshlet_thread_meshlet_ids,
        meshlets, meshlet_vertex_ids,
        meshlet_vertex_data,
        meshlet_thread_instance_ids,
        meshlet_instance_uniforms,
        unpack_vertex
    },
    mesh_functions,
    mesh_types::MESH_FLAGS_SIGN_DETERMINANT_MODEL_3X3_BIT,
    view_transformations::position_world_to_clip,
}
#import bevy_render::maths::{affine_to_square, mat2x4_f32_to_mat3x3_unpack}

#ifdef PREPASS_PIPELINE
#import bevy_pbr::prepass_io::VertexOutput
#else
#import bevy_pbr::forward_io::VertexOutput
#endif

@vertex
fn meshlet_vertex(@builtin(vertex_index) packed_meshlet_index: u32) -> VertexOutput {
    let thread_id = packed_meshlet_index >> 8u;
    let meshlet_id = meshlet_thread_meshlet_ids[thread_id];
    let meshlet = meshlets[meshlet_id];
    let index = extractBits(packed_meshlet_index, 0u, 8u);
    let vertex_id = meshlet_vertex_ids[meshlet.start_vertex_id + index];
    let vertex = unpack_vertex(meshlet_vertex_data[vertex_id]);
    let instance_id = meshlet_thread_instance_ids[thread_id];
    let instance_uniform = meshlet_instance_uniforms[instance_id];

    var out: VertexOutput;
    let model = affine_to_square(instance_uniform.model);
    out.world_normal = normalize(
        mat2x4_f32_to_mat3x3_unpack(
            instance_uniform.inverse_transpose_model_a,
            instance_uniform.inverse_transpose_model_b,
        ) * vertex.normal
    );
    out.world_position = mesh_functions::mesh_position_local_to_world(model, vec4<f32>(vertex.position, 1.0));
    out.position = position_world_to_clip(out.world_position.xyz);
    out.uv = vertex.uv;
    out.world_tangent = vec4<f32>(
        normalize(
            mat3x3<f32>(
                model[0].xyz,
                model[1].xyz,
                model[2].xyz
            ) * vertex.tangent.xyz
        ),
        vertex.tangent.w * (f32(bool(instance_uniform.flags & MESH_FLAGS_SIGN_DETERMINANT_MODEL_3X3_BIT)) * 2.0 - 1.0)
    );

#ifdef MOTION_VECTOR_PREPASS
    out.previous_world_position = mesh_functions::mesh_position_local_to_world(
        affine_to_square(instance_uniform.previous_model),
        vec4<f32>(vertex.position, 1.0)
    );
#endif

#ifdef DEPTH_CLAMP_ORTHO
    out.clip_position_unclamped = out.position;
    out.position.z = min(out.position.z, 1.0);
#endif

#ifdef VERTEX_OUTPUT_MESH_FLAGS
    out.mesh_flags = instance_uniform.flags;
#endif

    return out;
}
