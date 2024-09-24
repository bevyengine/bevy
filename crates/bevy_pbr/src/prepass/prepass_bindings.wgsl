#define_import_path bevy_pbr::prepass_bindings

struct PreviousViewUniforms {
    view_from_world: mat4x4<f32>,
    clip_from_world: mat4x4<f32>,
}

#ifdef MOTION_VECTOR_PREPASS
@group(0) @binding(2) var<uniform> previous_view_uniforms: PreviousViewUniforms;
#endif // MOTION_VECTOR_PREPASS

// Material bindings will be in @group(2)
