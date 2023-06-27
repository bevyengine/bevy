#import bevy_pbr::prepass_bindings
#import bevy_pbr::mesh_functions
#import bevy_pbr::prepass_io

#ifdef DEFERRED_PREPASS
#import bevy_pbr::rgb9e5
#endif

#ifdef MORPH_TARGETS
fn morph_vertex(vertex_in: Vertex) -> Vertex {
    var vertex = vertex_in;
    let weight_count = layer_count();
    for (var i: u32 = 0u; i < weight_count; i ++) {
        let weight = weight_at(i);
        if weight == 0.0 {
            continue;
        }
        vertex.position += weight * morph(vertex.index, position_offset, i);
#ifdef VERTEX_NORMALS
        vertex.normal += weight * morph(vertex.index, normal_offset, i);
#endif
#ifdef VERTEX_TANGENTS
        vertex.tangent += vec4(weight * morph(vertex.index, tangent_offset, i), 0.0);
#endif
    }
    return vertex;
}
#endif

@vertex
fn vertex(vertex_no_morph: Vertex) -> VertexOutput {
    var out: VertexOutput;

#ifdef MORPH_TARGETS
    var vertex = morph_vertex(vertex_no_morph);
#else
    var vertex = vertex_no_morph;
#endif

#ifdef SKINNED
    var model = skin_model(vertex.joint_indices, vertex.joint_weights);
#else // SKINNED
    var model = mesh.model;
#endif // SKINNED

    out.clip_position = mesh_position_local_to_clip(model, vec4(vertex.position, 1.0));
#ifdef DEPTH_CLAMP_ORTHO
    out.clip_position_unclamped = out.clip_position;
    out.clip_position.z = min(out.clip_position.z, 1.0);
#endif // DEPTH_CLAMP_ORTHO

#ifdef VERTEX_UVS
    out.uv = vertex.uv;
#endif // VERTEX_UVS

#ifdef NORMAL_PREPASS_OR_DEFERRED_PREPASS
#ifdef SKINNED
    out.world_normal = skin_normals(model, vertex.normal);
#else // SKINNED
    out.world_normal = mesh_normal_local_to_world(vertex.normal);
#endif // SKINNED

#ifdef VERTEX_TANGENTS
    out.world_tangent = mesh_tangent_local_to_world(model, vertex.tangent);
#endif // VERTEX_TANGENTS
#endif // NORMAL_PREPASS_OR_DEFERRED_PREPASS

    out.world_position = mesh_position_local_to_world(model, vec4<f32>(vertex.position, 1.0));
#ifdef MOTION_VECTOR_PREPASS
    out.previous_world_position = mesh_position_local_to_world(mesh.previous_model, vec4<f32>(vertex.position, 1.0));
#endif // MOTION_VECTOR_PREPASS

    return out;
}

#ifdef PREPASS_FRAGMENT
@fragment
fn fragment(in: FragmentInput) -> FragmentOutput {
    var out: FragmentOutput;

#ifdef NORMAL_PREPASS
    out.normal = vec4(in.world_normal * 0.5 + vec3(0.5), 1.0);
#endif

#ifdef DEPTH_CLAMP_ORTHO
    out.frag_depth = in.clip_position_unclamped.z;
#endif // DEPTH_CLAMP_ORTHO

#ifdef MOTION_VECTOR_PREPASS
    let clip_position_t = view.unjittered_view_proj * in.world_position;
    let clip_position = clip_position_t.xy / clip_position_t.w;
    let previous_clip_position_t = previous_view_proj * in.previous_world_position;
    let previous_clip_position = previous_clip_position_t.xy / previous_clip_position_t.w;
    // These motion vectors are used as offsets to UV positions and are stored
    // in the range -1,1 to allow offsetting from the one corner to the
    // diagonally-opposite corner in UV coordinates, in either direction.
    // A difference between diagonally-opposite corners of clip space is in the
    // range -2,2, so this needs to be scaled by 0.5. And the V direction goes
    // down where clip space y goes up, so y needs to be flipped.
    out.motion_vector = (clip_position - previous_clip_position) * vec2(0.5, -0.5);
#endif // MOTION_VECTOR_PREPASS

#ifdef DEFERRED_PREPASS
    out.deferred = vec4(0u, vec3_to_rgb9e5(vec3(1.0, 0.0, 1.0)), 0u, 0u);
#endif

    return out;
}
#endif // PREPASS_FRAGMENT
