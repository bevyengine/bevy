#define_import_path bevy_render::view

struct ColorGrading {
    exposure: f32,
    gamma: f32,
    pre_saturation: f32,
    post_saturation: f32,
}

struct View {
    world_to_clip: mat4x4<f32>,
    unjittered_world_to_clip: mat4x4<f32>,
    clip_to_world: mat4x4<f32>,
    view_to_world: mat4x4<f32>,
    world_to_view: mat4x4<f32>,
    view_to_clip: mat4x4<f32>,
    clip_to_view: mat4x4<f32>,
    world_position: vec3<f32>,
    // viewport(x_origin, y_origin, width, height)
    viewport: vec4<f32>,
    color_grading: ColorGrading,
};
