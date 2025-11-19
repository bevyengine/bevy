#define_import_path bevy_pbr::pbr_types

// Since this is a hot path, try to keep the alignment and size of the struct members in mind.
// You can find the alignment and sizes at <https://www.w3.org/TR/WGSL/#alignment-and-size>.
struct StandardMaterial {
    base_color: vec4<f32>,
    emissive: vec4<f32>,
    attenuation_color: vec4<f32>,
    uv_transform: mat3x3<f32>,
    reflectance: vec3<f32>,
    perceptual_roughness: f32,
    metallic: f32,
    diffuse_transmission: f32,
    specular_transmission: f32,
    thickness: f32,
    ior: f32,
    attenuation_distance: f32,
    clearcoat: f32,
    clearcoat_perceptual_roughness: f32,
    anisotropy_strength: f32,
    anisotropy_rotation: vec2<f32>,
    // 'flags' is a bit field indicating various options. u32 is 32 bits so we have up to 32 options.
    flags: u32,
    alpha_cutoff: f32,
    parallax_depth_scale: f32,
    max_parallax_layer_count: f32,
    lightmap_exposure: f32,
    max_relief_mapping_search_steps: u32,
    /// ID for specifying which deferred lighting pass should be used for rendering this material, if any.
    deferred_lighting_pass_id: u32,
};

// !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!
// NOTE: if these flags are updated or changed. Be sure to also update
// deferred_flags_from_mesh_material_flags and mesh_material_flags_from_deferred_flags
// !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!
const STANDARD_MATERIAL_FLAGS_BASE_COLOR_TEXTURE_BIT: u32            = 1u << 0u;
const STANDARD_MATERIAL_FLAGS_EMISSIVE_TEXTURE_BIT: u32              = 1u << 1u;
const STANDARD_MATERIAL_FLAGS_METALLIC_ROUGHNESS_TEXTURE_BIT: u32    = 1u << 2u;
const STANDARD_MATERIAL_FLAGS_OCCLUSION_TEXTURE_BIT: u32             = 1u << 3u;
const STANDARD_MATERIAL_FLAGS_DOUBLE_SIDED_BIT: u32                  = 1u << 4u;
const STANDARD_MATERIAL_FLAGS_UNLIT_BIT: u32                         = 1u << 5u;
const STANDARD_MATERIAL_FLAGS_TWO_COMPONENT_NORMAL_MAP: u32          = 1u << 6u;
const STANDARD_MATERIAL_FLAGS_FLIP_NORMAL_MAP_Y: u32                 = 1u << 7u;
const STANDARD_MATERIAL_FLAGS_FOG_ENABLED_BIT: u32                   = 1u << 8u;
const STANDARD_MATERIAL_FLAGS_DEPTH_MAP_BIT: u32                     = 1u << 9u;
const STANDARD_MATERIAL_FLAGS_SPECULAR_TRANSMISSION_TEXTURE_BIT: u32 = 1u << 10u;
const STANDARD_MATERIAL_FLAGS_THICKNESS_TEXTURE_BIT: u32             = 1u << 11u;
const STANDARD_MATERIAL_FLAGS_DIFFUSE_TRANSMISSION_TEXTURE_BIT: u32  = 1u << 12u;
const STANDARD_MATERIAL_FLAGS_ATTENUATION_ENABLED_BIT: u32           = 1u << 13u;
const STANDARD_MATERIAL_FLAGS_CLEARCOAT_TEXTURE_BIT: u32             = 1u << 14u;
const STANDARD_MATERIAL_FLAGS_CLEARCOAT_ROUGHNESS_TEXTURE_BIT: u32   = 1u << 15u;
const STANDARD_MATERIAL_FLAGS_CLEARCOAT_NORMAL_TEXTURE_BIT: u32      = 1u << 16u;
const STANDARD_MATERIAL_FLAGS_ANISOTROPY_TEXTURE_BIT: u32            = 1u << 17u;
const STANDARD_MATERIAL_FLAGS_SPECULAR_TEXTURE_BIT: u32              = 1u << 18u;
const STANDARD_MATERIAL_FLAGS_SPECULAR_TINT_TEXTURE_BIT: u32         = 1u << 19u;
const STANDARD_MATERIAL_FLAGS_ALPHA_MODE_RESERVED_BITS: u32          = 7u << 29u; // (0b111u << 29u)
const STANDARD_MATERIAL_FLAGS_ALPHA_MODE_OPAQUE: u32                 = 0u << 29u;
const STANDARD_MATERIAL_FLAGS_ALPHA_MODE_MASK: u32                   = 1u << 29u;
const STANDARD_MATERIAL_FLAGS_ALPHA_MODE_BLEND: u32                  = 2u << 29u;
const STANDARD_MATERIAL_FLAGS_ALPHA_MODE_PREMULTIPLIED: u32          = 3u << 29u;
const STANDARD_MATERIAL_FLAGS_ALPHA_MODE_ADD: u32                    = 4u << 29u;
const STANDARD_MATERIAL_FLAGS_ALPHA_MODE_MULTIPLY: u32               = 5u << 29u;
const STANDARD_MATERIAL_FLAGS_ALPHA_MODE_ALPHA_TO_COVERAGE: u32      = 6u << 29u;


// Creates a StandardMaterial with default values
fn standard_material_new() -> StandardMaterial {
    var material: StandardMaterial;

    // NOTE: Keep in-sync with src/pbr_material.rs!
    material.base_color = vec4<f32>(1.0, 1.0, 1.0, 1.0);
    material.emissive = vec4<f32>(0.0, 0.0, 0.0, 1.0);
    material.perceptual_roughness = 0.5;
    material.metallic = 0.00;
    material.reflectance = vec3<f32>(0.5);
    material.diffuse_transmission = 0.0;
    material.specular_transmission = 0.0;
    material.thickness = 0.0;
    material.ior = 1.5;
    material.attenuation_distance = 1.0;
    material.attenuation_color = vec4<f32>(1.0, 1.0, 1.0, 1.0);
    material.clearcoat = 0.0;
    material.clearcoat_perceptual_roughness = 0.0;
    material.flags = STANDARD_MATERIAL_FLAGS_ALPHA_MODE_OPAQUE;
    material.alpha_cutoff = 0.5;
    material.parallax_depth_scale = 0.1;
    material.max_parallax_layer_count = 16.0;
    material.max_relief_mapping_search_steps = 5u;
    material.deferred_lighting_pass_id = 1u;
    // scale 1, translation 0, rotation 0
    material.uv_transform = mat3x3<f32>(1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0);

    return material;
}

struct PbrInput {
    material: StandardMaterial,
    // Note: this gets monochromized upon deferred PbrInput reconstruction.
    diffuse_occlusion: vec3<f32>,
    // Note: this is 1.0 (entirely unoccluded) when SSAO and SSR are off.
    specular_occlusion: f32,
    frag_coord: vec4<f32>,
    world_position: vec4<f32>,
    // Normalized world normal used for shadow mapping as normal-mapping is not used for shadow
    // mapping
    world_normal: vec3<f32>,
    // Normalized normal-mapped world normal used for lighting
    N: vec3<f32>,
    // Normalized view vector in world space, pointing from the fragment world position toward the
    // view world position
    V: vec3<f32>,
    lightmap_light: vec3<f32>,
    clearcoat_N: vec3<f32>,
    anisotropy_strength: f32,
    // These two aren't specific to anisotropy, but we only fill them in if
    // we're doing anisotropy, so they're prefixed with `anisotropy_`.
    anisotropy_T: vec3<f32>,
    anisotropy_B: vec3<f32>,
    is_orthographic: bool,
    flags: u32,
};

// Creates a PbrInput with default values
fn pbr_input_new() -> PbrInput {
    var pbr_input: PbrInput;

    pbr_input.material = standard_material_new();
    pbr_input.diffuse_occlusion = vec3<f32>(1.0);
    // If SSAO is enabled, then this gets overwritten with proper specular occlusion. If its not, then we get specular environment map unoccluded (we have no data with which to occlude it with).
    pbr_input.specular_occlusion = 1.0;

    pbr_input.frag_coord = vec4<f32>(0.0, 0.0, 0.0, 1.0);
    pbr_input.world_position = vec4<f32>(0.0, 0.0, 0.0, 1.0);
    pbr_input.world_normal = vec3<f32>(0.0, 0.0, 1.0);

    pbr_input.is_orthographic = false;

    pbr_input.N = vec3<f32>(0.0, 0.0, 1.0);
    pbr_input.V = vec3<f32>(1.0, 0.0, 0.0);

    pbr_input.clearcoat_N = vec3<f32>(0.0);
    pbr_input.anisotropy_T = vec3<f32>(0.0);
    pbr_input.anisotropy_B = vec3<f32>(0.0);

    pbr_input.lightmap_light = vec3<f32>(0.0);

    pbr_input.flags = 0u;

    return pbr_input;
}
