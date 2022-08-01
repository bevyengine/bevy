// NOTE: Keep in sync with pbr.wgsl
[[block]]
struct View {
    view_proj: mat4x4<f32>;
    projection: mat4x4<f32>;
    world_position: vec3<f32>;
};
[[group(0), binding(0)]]
var<uniform> view: View;

[[block]]
struct Mesh {
    model: mat4x4<f32>;
};
[[group(2), binding(0)]]
var<uniform> mesh: Mesh;

struct VertexOpaque {
    [[location(0)]] position: vec3<f32>;
};

struct VertexOutputOpaque {
    [[builtin(position)]] clip_position: vec4<f32>;
};

[[stage(vertex)]]
fn vertex_opaque(vertex: VertexOpaque) -> VertexOutputOpaque {
    // NOTE: The clip position MUST be calculated EXACTLY as it is in the
    //       pbr.wgsl vertex stage as the depth buffer must be EXACTLY
    //       equal!
    let world_position = mesh.model * vec4<f32>(vertex.position, 1.0);

    var out: VertexOutputOpaque;
    out.clip_position = view.view_proj * world_position;
    return out;
}

struct VertexAlphaMask {
    [[location(0)]] position: vec3<f32>;
    [[location(1)]] uv: vec2<f32>;
};

struct VertexOutputAlphaMask {
    [[builtin(position)]] clip_position: vec4<f32>;
    [[location(0)]] uv: vec2<f32>;
};

[[stage(vertex)]]
fn vertex_alpha_mask(vertex: VertexAlphaMask) -> VertexOutputAlphaMask {
    // NOTE: The clip position MUST be calculated EXACTLY as it is in the
    //       pbr.wgsl vertex stage as the depth buffer must be EXACTLY
    //       equal!
    let world_position = mesh.model * vec4<f32>(vertex.position, 1.0);

    var out: VertexOutputAlphaMask;
    out.clip_position = view.view_proj * world_position;
    out.uv = vertex.uv;
    return out;
}

// NOTE: Keep in sync with pbr.wgsl!
[[block]]
struct StandardMaterial {
    base_color: vec4<f32>;
    emissive: vec4<f32>;
    perceptual_roughness: f32;
    metallic: f32;
    reflectance: f32;
    // 'flags' is a bit field indicating various options. u32 is 32 bits so we have up to 32 options.
    flags: u32;
    alpha_cutoff: f32;
};

let STANDARD_MATERIAL_FLAGS_BASE_COLOR_TEXTURE_BIT: u32         = 1u;
let STANDARD_MATERIAL_FLAGS_EMISSIVE_TEXTURE_BIT: u32           = 2u;
let STANDARD_MATERIAL_FLAGS_METALLIC_ROUGHNESS_TEXTURE_BIT: u32 = 4u;
let STANDARD_MATERIAL_FLAGS_OCCLUSION_TEXTURE_BIT: u32          = 8u;
let STANDARD_MATERIAL_FLAGS_DOUBLE_SIDED_BIT: u32               = 16u;
let STANDARD_MATERIAL_FLAGS_UNLIT_BIT: u32                      = 32u;
let STANDARD_MATERIAL_FLAGS_ALPHA_MODE_OPAQUE: u32              = 64u;
let STANDARD_MATERIAL_FLAGS_ALPHA_MODE_MASK: u32                = 128u;
let STANDARD_MATERIAL_FLAGS_ALPHA_MODE_BLEND: u32               = 256u;

[[group(1), binding(0)]]
var<uniform> material: StandardMaterial;
[[group(1), binding(1)]]
var base_color_texture: texture_2d<f32>;
[[group(1), binding(2)]]
var base_color_sampler: sampler;

struct FragmentInputAlphaMask {
    [[location(0)]] uv: vec2<f32>;
};

[[stage(fragment)]]
fn fragment_alpha_mask(in: FragmentInputAlphaMask) {
    var base_color: vec4<f32> = material.base_color;
    if ((material.flags & STANDARD_MATERIAL_FLAGS_BASE_COLOR_TEXTURE_BIT) != 0u) {
        base_color = base_color * textureSample(base_color_texture, base_color_sampler, in.uv);
    }
    if (base_color.a < material.alpha_cutoff) {
        discard;
    }
}