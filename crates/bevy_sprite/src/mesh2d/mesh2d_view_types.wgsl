#define_import_path bevy_sprite::mesh2d_view_types

struct View {
    view_proj: mat4x4<f32>,
    view: mat4x4<f32>,
    inverse_view: mat4x4<f32>,
    projection: mat4x4<f32>,
    world_position: vec3<f32>,
    width: f32,
    height: f32,
};
