#define_import_path bevy_sprite::sprite_types

struct SpriteMaterial {
    color: vec4<f32>,
    flags: u32,
    alpha_cutoff: f32,
    vertex_scale: vec2<f32>,
    vertex_offset: vec2<f32>,
    uv_transform: mat3x3<f32>,

    tile_stretch_value: vec2<f32>,

    scale: vec2<f32>,
    min_inset: vec2<f32>,
    max_inset: vec2<f32>,
    side_stretch_value: vec2<f32>,
    center_stretch_value: vec2<f32>,
};

const SPRITE_MATERIAL_FLAGS_ALPHA_MODE_RESERVED_BITS: u32 = 3221225472u; // (0b11u32 << 30)
const SPRITE_MATERIAL_FLAGS_ALPHA_MODE_OPAQUE: u32        = 0u;          // (0u32 << 30)
const SPRITE_MATERIAL_FLAGS_ALPHA_MODE_MASK: u32          = 1073741824u; // (1u32 << 30)
const SPRITE_MATERIAL_FLAGS_ALPHA_MODE_BLEND: u32         = 2147483648u; // (2u32 << 30)

const SPRITE_MATERIAL_FLAGS_FLIP_X: u32                   = 1u;
const SPRITE_MATERIAL_FLAGS_FLIP_Y: u32                   = 2u;
const SPRITE_MATERIAL_FLAGS_TILE_X: u32                   = 4u;
const SPRITE_MATERIAL_FLAGS_TILE_Y: u32                   = 8u;
