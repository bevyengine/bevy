#define_import_path bevy_core_pipeline::input_texture

#ifdef MULTIVIEW
@group(0) @binding(0) var in_texture: texture_2d_array<f32>;
#else
@group(0) @binding(0) var in_texture: texture_2d<f32>;
#endif