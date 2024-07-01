#import bevy_pbr::{
    prepass_bindings,
    mesh_functions,
    prepass_io::{Vertex, VertexOutput, FragmentOutput},
    skinning,
    morph,
    mesh_view_bindings::view,
    view_transformations::position_world_to_clip,
}

#ifdef DEFERRED_PREPASS
#import bevy_pbr::rgb9e5
#endif

#ifdef MORPH_TARGETS
fn morph_vertex(vertex_in: Vertex) -> Vertex {
    var vertex = vertex_in;
    let weight_count = morph::layer_count();
    for (var i: u32 = 0u; i < weight_count; i ++) {
        let weight = morph::weight_at(i);
        if weight == 0.0 {
            continue;
        }
        vertex.position += weight * morph::morph(vertex.index, morph::position_offset, i);
#ifdef VERTEX_NORMALS
        vertex.normal += weight * morph::morph(vertex.index, morph::normal_offset, i);
#endif
#ifdef VERTEX_TANGENTS
        vertex.tangent += vec4(weight * morph::morph(vertex.index, morph::tangent_offset, i), 0.0);
#endif
    }
    return vertex;
}

// Returns the morphed position of the given vertex from the previous frame.
//
// This function is used for motion vector calculation, and, as such, it doesn't
// bother morphing the normals and tangents.
fn morph_prev_vertex(vertex_in: Vertex) -> Vertex {
    var vertex = vertex_in;
    let weight_count = morph::layer_count();
    for (var i: u32 = 0u; i < weight_count; i ++) {
        let weight = morph::prev_weight_at(i);
        if weight == 0.0 {
            continue;
        }
        vertex.position += weight * morph::morph(vertex.index, morph::position_offset, i);
        // Don't bother morphing normals and tangents; we don't need them for
        // motion vector calculation.
    }
    return vertex;
}
#endif  // MORPH_TARGETS

@vertex
fn vertex(vertex_no_morph: Vertex) -> VertexOutput {
    var out: VertexOutput;

#ifdef MORPH_TARGETS
    var vertex = morph_vertex(vertex_no_morph);
#else
    var vertex = vertex_no_morph;
#endif

#ifdef SKINNED
    var world_from_local = skinning::skin_model(vertex.joint_indices, vertex.joint_weights);
#else // SKINNED
    // Use vertex_no_morph.instance_index instead of vertex.instance_index to work around a wgpu dx12 bug.
    // See https://github.com/gfx-rs/naga/issues/2416
    var world_from_local = mesh_functions::get_world_from_local(vertex_no_morph.instance_index);
#endif // SKINNED

    out.world_position = mesh_functions::mesh_position_local_to_world(world_from_local, vec4<f32>(vertex.position, 1.0));
    out.position = position_world_to_clip(out.world_position.xyz);
#ifdef DEPTH_CLAMP_ORTHO
    out.clip_position_unclamped = out.position;
    out.position.z = min(out.position.z, 1.0);
#endif // DEPTH_CLAMP_ORTHO

#ifdef VERTEX_UVS_A
    out.uv = vertex.uv;
#endif // VERTEX_UVS_A

#ifdef VERTEX_UVS_B
    out.uv_b = vertex.uv_b;
#endif // VERTEX_UVS_B

#ifdef NORMAL_PREPASS_OR_DEFERRED_PREPASS
#ifdef SKINNED
    out.world_normal = skinning::skin_normals(world_from_local, vertex.normal);
#else // SKINNED
    out.world_normal = mesh_functions::mesh_normal_local_to_world(
        vertex.normal,
        // Use vertex_no_morph.instance_index instead of vertex.instance_index to work around a wgpu dx12 bug.
        // See https://github.com/gfx-rs/naga/issues/2416
        vertex_no_morph.instance_index
    );
#endif // SKINNED

#ifdef VERTEX_TANGENTS
    out.world_tangent = mesh_functions::mesh_tangent_local_to_world(
        world_from_local,
        vertex.tangent,
        // Use vertex_no_morph.instance_index instead of vertex.instance_index to work around a wgpu dx12 bug.
        // See https://github.com/gfx-rs/naga/issues/2416
        vertex_no_morph.instance_index
    );
#endif // VERTEX_TANGENTS
#endif // NORMAL_PREPASS_OR_DEFERRED_PREPASS

#ifdef VERTEX_COLORS
    out.color = vertex.color;
#endif

    // Compute the motion vector for TAA among other purposes. For this we need
    // to know where the vertex was last frame.
#ifdef MOTION_VECTOR_PREPASS

    // Take morph targets into account.
#ifdef MORPH_TARGETS

#ifdef HAS_PREVIOUS_MORPH
    let prev_vertex = morph_prev_vertex(vertex_no_morph);
#else   // HAS_PREVIOUS_MORPH
    let prev_vertex = vertex_no_morph;
#endif  // HAS_PREVIOUS_MORPH

#else   // MORPH_TARGETS
    let prev_vertex = vertex_no_morph;
#endif  // MORPH_TARGETS

    // Take skinning into account.
#ifdef SKINNED

#ifdef HAS_PREVIOUS_SKIN
    let prev_model = skinning::skin_prev_model(
        prev_vertex.joint_indices,
        prev_vertex.joint_weights,
    );
#else   // HAS_PREVIOUS_SKIN
    let prev_model = mesh_functions::get_previous_world_from_local(prev_vertex.instance_index);
#endif  // HAS_PREVIOUS_SKIN

#else   // SKINNED
    let prev_model = mesh_functions::get_previous_world_from_local(prev_vertex.instance_index);
#endif  // SKINNED

    out.previous_world_position = mesh_functions::mesh_position_local_to_world(
        prev_model,
        vec4<f32>(prev_vertex.position, 1.0)
    );
#endif // MOTION_VECTOR_PREPASS

#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
    // Use vertex_no_morph.instance_index instead of vertex.instance_index to work around a wgpu dx12 bug.
    // See https://github.com/gfx-rs/naga/issues/2416
    out.instance_index = vertex_no_morph.instance_index;
#endif

    return out;
}

#ifdef PREPASS_FRAGMENT
@fragment
fn fragment(in: VertexOutput) -> FragmentOutput {
    var out: FragmentOutput;

#ifdef NORMAL_PREPASS
    out.normal = vec4(in.world_normal * 0.5 + vec3(0.5), 1.0);
#endif

#ifdef DEPTH_CLAMP_ORTHO
    out.frag_depth = in.clip_position_unclamped.z;
#endif // DEPTH_CLAMP_ORTHO

#ifdef MOTION_VECTOR_PREPASS
    let clip_position_t = view.unjittered_clip_from_world * in.world_position;
    let clip_position = clip_position_t.xy / clip_position_t.w;
    let previous_clip_position_t = prepass_bindings::previous_view_uniforms.clip_from_world * in.previous_world_position;
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
    // There isn't any material info available for this default prepass shader so we are just writing 
    // emissive magenta out to the deferred gbuffer to be rendered by the first deferred lighting pass layer.
    // This is here so if the default prepass fragment is used for deferred magenta will be rendered, and also
    // as an example to show that a user could write to the deferred gbuffer if they were to start from this shader.
    out.deferred = vec4(0u, bevy_pbr::rgb9e5::vec3_to_rgb9e5_(vec3(1.0, 0.0, 1.0)), 0u, 0u);
    out.deferred_lighting_pass_id = 1u;
#endif

    return out;
}
#endif // PREPASS_FRAGMENT
