#define_import_path bevy_pbr::mesh_view_types

struct View {
    view_proj: mat4x4<f32>;
    inverse_view_proj: mat4x4<f32>;
    view: mat4x4<f32>;
    inverse_view: mat4x4<f32>;
    projection: mat4x4<f32>;
    inverse_projection: mat4x4<f32>;
    world_position: vec3<f32>;
    width: f32;
    height: f32;
};

struct PointLight {
    // NOTE: [2][2] [2][3] [3][2] [3][3]
    projection_lr: vec4<f32>;
    color_inverse_square_range: vec4<f32>;
    position_radius: vec4<f32>;
    // 'flags' is a bit field indicating various options. u32 is 32 bits so we have up to 32 options.
    flags: u32;
    shadow_depth_bias: f32;
    shadow_normal_bias: f32;
};

let POINT_LIGHT_FLAGS_SHADOWS_ENABLED_BIT: u32 = 1u;

struct DirectionalLight {
    view_projection: mat4x4<f32>;
    color: vec4<f32>;
    direction_to_light: vec3<f32>;
    // 'flags' is a bit field indicating various options. u32 is 32 bits so we have up to 32 options.
    flags: u32;
    shadow_depth_bias: f32;
    shadow_normal_bias: f32;
};

let DIRECTIONAL_LIGHT_FLAGS_SHADOWS_ENABLED_BIT: u32 = 1u;

struct Lights {
    // NOTE: this array size must be kept in sync with the constants defined bevy_pbr2/src/render/light.rs
    directional_lights: array<DirectionalLight, 1u>;
    ambient_color: vec4<f32>;
    // x/y/z dimensions and n_clusters in w
    cluster_dimensions: vec4<u32>;
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
    cluster_factors: vec4<f32>;
    n_directional_lights: u32;
};

#ifdef NO_STORAGE_BUFFERS_SUPPORT
struct PointLights {
    data: array<PointLight, 256u>;
};
struct ClusterLightIndexLists {
    // each u32 contains 4 u8 indices into the PointLights array
    data: array<vec4<u32>, 1024u>;
};
struct ClusterOffsetsAndCounts {
    // each u32 contains a 24-bit index into ClusterLightIndexLists in the high 24 bits
    // and an 8-bit count of the number of lights in the low 8 bits
    data: array<vec4<u32>, 1024u>;
};
#else
struct PointLights {
    data: array<PointLight>;
};
struct ClusterLightIndexLists {
    data: array<u32>;
};
struct ClusterOffsetsAndCounts {
    data: array<vec2<u32>>;
};
#endif
