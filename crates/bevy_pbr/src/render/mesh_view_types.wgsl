#define_import_path bevy_pbr::mesh_view_types

struct ClusterableObject {
    // For point lights: the lower-right 2x2 values of the projection matrix [2][2] [2][3] [3][2] [3][3]
    // For spot lights: the direction (x,z), spot_scale and spot_offset
    light_custom_data: vec4<f32>,
    color_inverse_square_range: vec4<f32>,
    position_radius: vec4<f32>,
    // 'flags' is a bit field indicating various options. u32 is 32 bits so we have up to 32 options.
    flags: u32,
    shadow_depth_bias: f32,
    shadow_normal_bias: f32,
    spot_light_tan_angle: f32,
    soft_shadow_size: f32,
    shadow_map_near_z: f32,
    decal_index: u32,
    pad: f32,
};

const POINT_LIGHT_FLAGS_SHADOWS_ENABLED_BIT: u32                    = 1u << 0u;
const POINT_LIGHT_FLAGS_SPOT_LIGHT_Y_NEGATIVE: u32                  = 1u << 1u;
const POINT_LIGHT_FLAGS_VOLUMETRIC_BIT: u32                         = 1u << 2u;
const POINT_LIGHT_FLAGS_AFFECTS_LIGHTMAPPED_MESH_DIFFUSE_BIT: u32   = 1u << 3u;

struct DirectionalCascade {
    clip_from_world: mat4x4<f32>,
    texel_size: f32,
    far_bound: f32,
}

struct DirectionalLight {
    cascades: array<DirectionalCascade, #{MAX_CASCADES_PER_LIGHT}>,
    color: vec4<f32>,
    direction_to_light: vec3<f32>,
    // 'flags' is a bit field indicating various options. u32 is 32 bits so we have up to 32 options.
    flags: u32,
    soft_shadow_size: f32,
    shadow_depth_bias: f32,
    shadow_normal_bias: f32,
    num_cascades: u32,
    cascades_overlap_proportion: f32,
    depth_texture_base_index: u32,
    decal_index: u32,
};

const DIRECTIONAL_LIGHT_FLAGS_SHADOWS_ENABLED_BIT: u32                  = 1u << 0u;
const DIRECTIONAL_LIGHT_FLAGS_VOLUMETRIC_BIT: u32                       = 1u << 1u;
const DIRECTIONAL_LIGHT_FLAGS_AFFECTS_LIGHTMAPPED_MESH_DIFFUSE_BIT: u32 = 1u << 2u;

struct Lights {
    // NOTE: this array size must be kept in sync with the constants defined in bevy_pbr/src/render/light.rs
    directional_lights: array<DirectionalLight, #{MAX_DIRECTIONAL_LIGHTS}u>,
    ambient_color: vec4<f32>,
    // x/y/z dimensions and n_clusters in w
    cluster_dimensions: vec4<u32>,
    // xy are vec2<f32>(cluster_dimensions.xy) / vec2<f32>(view.width, view.height)
    //
    // For perspective projections:
    // z is cluster_dimensions.z / log(far / near)
    // w is cluster_dimensions.z * log(near) / log(far / near)
    //
    // For orthographic projections:
    // NOTE: near and far are +ve but -z is infront of the camera
    // z is -near
    // w is cluster_dimensions.z / (-far - -near)
    cluster_factors: vec4<f32>,
    n_directional_lights: u32,
    spot_light_shadowmap_offset: i32,
    ambient_light_affects_lightmapped_meshes: u32
};

struct Fog {
    base_color: vec4<f32>,
    directional_light_color: vec4<f32>,
    // `be` and `bi` are allocated differently depending on the fog mode
    //
    // For Linear Fog:
    //     be.x = start, be.y = end
    // For Exponential and ExponentialSquared Fog:
    //     be.x = density
    // For Atmospheric Fog:
    //     be = per-channel extinction density
    //     bi = per-channel inscattering density
    be: vec3<f32>,
    directional_light_exponent: f32,
    bi: vec3<f32>,
    mode: u32,
}

// Important: These must be kept in sync with `fog.rs`
const FOG_MODE_OFF: u32                   = 0u;
const FOG_MODE_LINEAR: u32                = 1u;
const FOG_MODE_EXPONENTIAL: u32           = 2u;
const FOG_MODE_EXPONENTIAL_SQUARED: u32   = 3u;
const FOG_MODE_ATMOSPHERIC: u32           = 4u;

#if AVAILABLE_STORAGE_BUFFER_BINDINGS >= 3
struct ClusterableObjects {
    data: array<ClusterableObject>,
};
struct ClusterLightIndexLists {
    data: array<u32>,
};
struct ClusterOffsetsAndCounts {
    data: array<array<vec4<u32>, 2>>,
};
#else
struct ClusterableObjects {
    data: array<ClusterableObject, 204u>,
};
struct ClusterLightIndexLists {
    // each u32 contains 4 u8 indices into the ClusterableObjects array
    data: array<vec4<u32>, 1024u>,
};
struct ClusterOffsetsAndCounts {
    // each u32 contains a 24-bit index into ClusterLightIndexLists in the high 24 bits
    // and an 8-bit count of the number of lights in the low 8 bits
    data: array<vec4<u32>, 1024u>,
};
#endif

struct LightProbe {
    // This is stored as the transpose in order to save space in this structure.
    // It'll be transposed in the `environment_map_light` function.
    light_from_world_transposed: mat3x4<f32>,
    cubemap_index: i32,
    intensity: f32,
    // Whether this light probe contributes diffuse light to lightmapped meshes.
    affects_lightmapped_mesh_diffuse: u32,
};

struct LightProbes {
    // This must match `MAX_VIEW_REFLECTION_PROBES` on the Rust side.
    reflection_probes: array<LightProbe, 8u>,
    irradiance_volumes: array<LightProbe, 8u>,
    reflection_probe_count: i32,
    irradiance_volume_count: i32,
    // The index of the view environment map cubemap binding, or -1 if there's
    // no such cubemap.
    view_cubemap_index: i32,
    // The smallest valid mipmap level for the specular environment cubemap
    // associated with the view.
    smallest_specular_mip_level_for_view: u32,
    // The intensity of the environment map associated with the view.
    intensity_for_view: f32,
    // Whether the environment map attached to the view affects the diffuse
    // lighting for lightmapped meshes.
    view_environment_map_affects_lightmapped_mesh_diffuse: u32,
};

// Settings for screen space reflections.
//
// For more information on these settings, see the documentation for
// `bevy_pbr::ssr::ScreenSpaceReflections`.
struct ScreenSpaceReflectionsSettings {
    perceptual_roughness_threshold: f32,
    thickness: f32,
    linear_steps: u32,
    linear_march_exponent: f32,
    bisection_steps: u32,
    use_secant: u32,
};

struct EnvironmentMapUniform {
    // Transformation matrix for the environment cubemaps in world space.
    transform: mat4x4<f32>,
};

// Shader version of the order independent transparency settings component.
struct OrderIndependentTransparencySettings {
  layers_count: i32,
  alpha_threshold: f32,
};

struct ClusteredDecal {
    local_from_world: mat4x4<f32>,
    image_index: i32,
    tag: u32,
    pad_a: u32,
    pad_b: u32,
}

struct ClusteredDecals {
    decals: array<ClusteredDecal>,
}
