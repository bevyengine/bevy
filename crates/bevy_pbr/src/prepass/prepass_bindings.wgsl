#define_import_path bevy_pbr::prepass_bindings

struct PreviousViewUniforms {
    view_from_world: mat4x4<f32>,
    clip_from_world: mat4x4<f32>,
    clip_from_view: mat4x4<f32>,
    world_from_clip: mat4x4<f32>,
    view_from_clip: mat4x4<f32>,
}

@group(0) @binding(2) var<uniform> previous_view_uniforms: PreviousViewUniforms;

// Material bindings will be in @group(2)
