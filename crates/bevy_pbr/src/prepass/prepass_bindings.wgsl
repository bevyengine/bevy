#define_import_path bevy_pbr::prepass_bindings

struct PreviousViewUniforms {
    inverse_view: mat4x4<f32>,
    view_proj: mat4x4<f32>,
}

#ifdef MOTION_VECTOR_PREPASS
@group(0) @binding(2) var<uniform> previous_view_uniforms: PreviousViewUniforms;
#endif // MOTION_VECTOR_PREPASS

// Zero if the current mesh did not have skin/morph data available last frame, else one
#ifdef ANIMATED_MESH_MOTION_VECTORS
var<push_constant> motion_vectors_mask: f32;
#endif

// Material bindings will be in @group(2)
