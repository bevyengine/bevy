#define_import_path bevy_pbr::prepass_io

// Most of these attributes are not used in the default prepass fragment shader, but they are still needed so we can
// pass them to custom prepass shaders like pbr_prepass.wgsl.
struct Vertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,

#ifdef VERTEX_UVS
    @location(1) uv: vec2<f32>,
#endif

#ifdef NORMAL_PREPASS_OR_DEFERRED_PREPASS
    @location(2) normal: vec3<f32>,
#ifdef VERTEX_TANGENTS
    @location(3) tangent: vec4<f32>,
#endif
#endif // NORMAL_PREPASS_OR_DEFERRED_PREPASS

#ifdef SKINNED
    @location(4) joint_indices: vec4<u32>,
    @location(5) joint_weights: vec4<f32>,
#endif

#ifdef VERTEX_COLORS
    @location(6) color: vec4<f32>,
#endif

#ifdef MORPH_TARGETS
    @builtin(vertex_index) index: u32,
#endif // MORPH_TARGETS
}

struct VertexOutput {
    // This is `clip position` when the struct is used as a vertex stage output
    // and `frag coord` when used as a fragment stage input
    @builtin(position) position: vec4<f32>,

#ifdef VERTEX_UVS
    @location(0) uv: vec2<f32>,
#endif

#ifdef NORMAL_PREPASS_OR_DEFERRED_PREPASS
    @location(1) world_normal: vec3<f32>,
#ifdef VERTEX_TANGENTS
    @location(2) world_tangent: vec4<f32>,
#endif
#endif // NORMAL_PREPASS_OR_DEFERRED_PREPASS

    @location(3) world_position: vec4<f32>,
#ifdef MOTION_VECTOR_PREPASS
    @location(4) previous_world_position: vec4<f32>,
#endif

#ifdef DEPTH_CLAMP_ORTHO
    @location(5) clip_position_unclamped: vec4<f32>,
#endif // DEPTH_CLAMP_ORTHO
#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
    @location(6) instance_index: u32,
#endif

#ifdef VERTEX_COLORS
    @location(7) color: vec4<f32>,
#endif
}

#ifdef PREPASS_FRAGMENT
struct FragmentOutput {
#ifdef NORMAL_PREPASS
    @location(0) normal: vec4<f32>,
#endif

#ifdef MOTION_VECTOR_PREPASS
    @location(1) motion_vector: vec2<f32>,
#endif

#ifdef DEFERRED_PREPASS
    @location(2) deferred: vec4<u32>,
    @location(3) deferred_lighting_pass_id: u32,
#endif

#ifdef DEPTH_CLAMP_ORTHO
    @builtin(frag_depth) frag_depth: f32,
#endif // DEPTH_CLAMP_ORTHO
}
#endif //PREPASS_FRAGMENT
