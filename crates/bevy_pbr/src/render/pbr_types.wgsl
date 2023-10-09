#define_import_path bevy_pbr::pbr_types

struct StandardMaterial {
    base_color: vec4<f32>,
    emissive: vec4<f32>,
    perceptual_roughness: f32,
    metallic: f32,
    reflectance: f32,
    diffuse_transmission: f32,
    specular_transmission: f32,
    thickness: f32,
    ior: f32,
    attenuation_distance: f32,
    attenuation_color: vec4<f32>,
    // 'flags' is a bit field indicating various options. u32 is 32 bits so we have up to 32 options.
    flags: u32,
    alpha_cutoff: f32,
    parallax_depth_scale: f32,
    max_parallax_layer_count: f32,
    max_relief_mapping_search_steps: u32,
};

const STANDARD_MATERIAL_FLAGS_BASE_COLOR_TEXTURE_BIT: u32         = 1u;
const STANDARD_MATERIAL_FLAGS_EMISSIVE_TEXTURE_BIT: u32           = 2u;
const STANDARD_MATERIAL_FLAGS_METALLIC_ROUGHNESS_TEXTURE_BIT: u32 = 4u;
const STANDARD_MATERIAL_FLAGS_OCCLUSION_TEXTURE_BIT: u32          = 8u;
const STANDARD_MATERIAL_FLAGS_DOUBLE_SIDED_BIT: u32               = 16u;
const STANDARD_MATERIAL_FLAGS_UNLIT_BIT: u32                      = 32u;
const STANDARD_MATERIAL_FLAGS_TWO_COMPONENT_NORMAL_MAP: u32       = 64u;
const STANDARD_MATERIAL_FLAGS_FLIP_NORMAL_MAP_Y: u32              = 128u;
const STANDARD_MATERIAL_FLAGS_FOG_ENABLED_BIT: u32                = 256u;
const STANDARD_MATERIAL_FLAGS_DEPTH_MAP_BIT: u32                  = 512u;
const STANDARD_MATERIAL_FLAGS_SPECULAR_TRANSMISSION_TEXTURE_BIT: u32 = 1024u;
const STANDARD_MATERIAL_FLAGS_THICKNESS_TEXTURE_BIT: u32          = 2048u;
const STANDARD_MATERIAL_FLAGS_DIFFUSE_TRANSMISSION_TEXTURE_BIT: u32 = 4096u;
const STANDARD_MATERIAL_FLAGS_ATTENUATION_ENABLED_BIT: u32        = 8192u;
const STANDARD_MATERIAL_FLAGS_ALPHA_MODE_RESERVED_BITS: u32       = 3758096384u; // (0b111u32 << 29)
const STANDARD_MATERIAL_FLAGS_ALPHA_MODE_OPAQUE: u32              = 0u;          // (0u32 << 29)
const STANDARD_MATERIAL_FLAGS_ALPHA_MODE_MASK: u32                = 536870912u;  // (1u32 << 29)
const STANDARD_MATERIAL_FLAGS_ALPHA_MODE_BLEND: u32               = 1073741824u; // (2u32 << 29)
const STANDARD_MATERIAL_FLAGS_ALPHA_MODE_PREMULTIPLIED: u32       = 1610612736u; // (3u32 << 29)
const STANDARD_MATERIAL_FLAGS_ALPHA_MODE_ADD: u32                 = 2147483648u; // (4u32 << 29)
const STANDARD_MATERIAL_FLAGS_ALPHA_MODE_MULTIPLY: u32            = 2684354560u; // (5u32 << 29)
// ↑ To calculate/verify the values above, use the following playground:
// https://play.rust-lang.org/?version=stable&mode=debug&edition=2021&gist=7792f8dd6fc6a8d4d0b6b1776898a7f4

// Creates a StandardMaterial with default values
fn standard_material_new() -> StandardMaterial {
    var material: StandardMaterial;

    // NOTE: Keep in-sync with src/pbr_material.rs!
    material.base_color = vec4<f32>(1.0, 1.0, 1.0, 1.0);
    material.emissive = vec4<f32>(0.0, 0.0, 0.0, 1.0);
    material.perceptual_roughness = 0.5;
    material.metallic = 0.00;
    material.reflectance = 0.5;
    material.diffuse_transmission = 0.0;
    material.specular_transmission = 0.0;
    material.thickness = 0.0;
    material.ior = 1.5;
    material.attenuation_distance = 1.0;
    material.attenuation_color = vec4<f32>(1.0, 1.0, 1.0, 1.0);
    material.flags = STANDARD_MATERIAL_FLAGS_ALPHA_MODE_OPAQUE;
    material.alpha_cutoff = 0.5;
    material.parallax_depth_scale = 0.1;
    material.max_parallax_layer_count = 16.0;
    material.max_relief_mapping_search_steps = 5u;

    return material;
}
