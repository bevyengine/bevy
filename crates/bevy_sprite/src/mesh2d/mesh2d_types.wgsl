#define_import_path bevy_sprite::mesh2d_types

struct Mesh2d {
    model: mat4x4<f32>,
    inverse_transpose_model: mat4x4<f32>,
    // 'flags' is a bit field indicating various options. u32 is 32 bits so we have up to 32 options.
    flags: u32,
};
