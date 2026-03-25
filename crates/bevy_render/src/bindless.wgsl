// Defines the common arrays used to access bindless resources.
//
// This need to be kept up to date with the `BINDING_NUMBERS` table in
// `bindless.rs`.
//
// You access these by indexing into the bindless index table, and from there
// indexing into the appropriate binding array. For example, to access the base
// color texture of a `StandardMaterial` in bindless mode, write
// `bindless_textures_2d[materials[slot].base_color_texture]`, where
// `materials` is the bindless index table and `slot` is the index into that
// table (which can be found in the `Mesh`).

#define_import_path bevy_render::bindless

#ifdef BINDLESS

// Binding 0 is the bindless index table.
// Filtering samplers.
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var bindless_samplers_filtering: binding_array<sampler>;
// Non-filtering samplers (nearest neighbor).
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var bindless_samplers_non_filtering: binding_array<sampler>;
// Comparison samplers (typically for shadow mapping).
@group(#{MATERIAL_BIND_GROUP}) @binding(3) var bindless_samplers_comparison: binding_array<sampler>;
// 1D textures.
@group(#{MATERIAL_BIND_GROUP}) @binding(4) var bindless_textures_1d: binding_array<texture_1d<f32>>;
// 2D textures.
@group(#{MATERIAL_BIND_GROUP}) @binding(5) var bindless_textures_2d: binding_array<texture_2d<f32>>;
// 2D array textures.
@group(#{MATERIAL_BIND_GROUP}) @binding(6) var bindless_textures_2d_array: binding_array<texture_2d_array<f32>>;
// 3D textures.
@group(#{MATERIAL_BIND_GROUP}) @binding(7) var bindless_textures_3d: binding_array<texture_3d<f32>>;
// Cubemap textures.
@group(#{MATERIAL_BIND_GROUP}) @binding(8) var bindless_textures_cube: binding_array<texture_cube<f32>>;
// Cubemap array textures.
@group(#{MATERIAL_BIND_GROUP}) @binding(9) var bindless_textures_cube_array: binding_array<texture_cube_array<f32>>;

#endif  // BINDLESS
