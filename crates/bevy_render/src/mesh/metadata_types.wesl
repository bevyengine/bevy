
struct MeshMetadata {
    /// AABB for decompressing positions.
    aabb_center: vec3<f32>,
    /// `MeshAttributeCompressionFlags`, with the bits below.
    attribute_compression: u32,
    aabb_half_extents: vec3<f32>,
    pad_b: u32,
    /// UV channels range for decompressing UVs coordinates.
    uv_channels_min_and_extents: array<vec4<f32>, 2>,
};

const COMPRESS_POSITION: u32 = 1u;
const COMPRESS_NORMAL: u32 = 2u;
const COMPRESS_TANGENT: u32 = 4u;
const COMPRESS_UV0: u32 = 8u;
const COMPRESS_UV1: u32 = 16u;
