use crate::{
    render::{
        MeshLayouts, MeshPipelineViewLayout, MeshPipelineViewLayoutKey, MeshPipelineViewLayouts,
    },
    render_resource::*,
};
use bevy_asset::Handle;
use bevy_ecs::resource::Resource;
use bevy_mesh::BaseMeshPipelineKey;
use bevy_shader::Shader;

use static_assertions::const_assert_eq;

/// How many textures are allowed in the view bind group layout (`@group(0)`) before
/// broader compatibility with WebGL and WebGPU is at risk, due to the minimum guaranteed
/// values for `MAX_TEXTURE_IMAGE_UNITS` (in WebGL) and `maxSampledTexturesPerShaderStage` (in WebGPU),
/// currently both at 16.
///
/// We use 10 here because it still leaves us, in a worst case scenario, with 6 textures for the other bind groups.
///
/// See: <https://gpuweb.github.io/gpuweb/#limits>
#[cfg(debug_assertions)]
pub const MESH_PIPELINE_VIEW_LAYOUT_SAFE_MAX_TEXTURES: usize = 10;

/// All data needed to construct a pipeline for rendering 3D meshes.
#[derive(Resource, Clone)]
pub struct MeshPipeline {
    /// A reference to all the mesh pipeline view layouts.
    pub view_layouts: MeshPipelineViewLayouts,
    pub clustered_forward_buffer_binding_type: BufferBindingType,
    pub mesh_layouts: MeshLayouts,
    /// The shader asset handle.
    pub shader: Handle<Shader>,
    /// `MeshUniform`s are stored in arrays in buffers. If storage buffers are available, they
    /// are used and this will be `None`, otherwise uniform buffers will be used with batches
    /// of this many `MeshUniform`s, stored at dynamic offsets within the uniform buffer.
    /// Use code like this in custom shaders:
    /// ```wgsl
    /// ##ifdef PER_OBJECT_BUFFER_BATCH_SIZE
    /// @group(1) @binding(0) var<uniform> mesh: array<Mesh, #{PER_OBJECT_BUFFER_BATCH_SIZE}u>;
    /// ##else
    /// @group(1) @binding(0) var<storage> mesh: array<Mesh>;
    /// ##endif // PER_OBJECT_BUFFER_BATCH_SIZE
    /// ```
    pub per_object_buffer_batch_size: Option<u32>,

    /// Whether binding arrays (a.k.a. bindless textures) are usable on the
    /// current render device.
    ///
    /// This affects whether reflection probes can be used.
    pub binding_arrays_are_usable: bool,

    /// Whether clustered decals are usable on the current render device.
    pub clustered_decals_are_usable: bool,

    /// Whether skins will use uniform buffers on account of storage buffers
    /// being unavailable on this platform.
    pub skins_use_uniform_buffers: bool,
}

impl MeshPipeline {
    pub fn get_view_layout(
        &self,
        layout_key: MeshPipelineViewLayoutKey,
    ) -> &MeshPipelineViewLayout {
        self.view_layouts.get_view_layout(layout_key)
    }
}

bitflags::bitflags! {
    #[derive(Default, Clone, Copy, Debug, PartialEq, Eq, Hash)]
    #[repr(transparent)]
    // NOTE: Apparently quadro drivers support up to 64x MSAA.
    /// MSAA uses the highest 3 bits for the MSAA log2(sample count) to support up to 128x MSAA.
    pub struct MeshPipelineKey: u64 {
        // Nothing
        const NONE                              = 0;

        // Inherited bits
        const MORPH_TARGETS                     = BaseMeshPipelineKey::MORPH_TARGETS.bits();

        // Flag bits
        const HDR                               = 1 << 0;
        const TONEMAP_IN_SHADER                 = 1 << 1;
        const DEBAND_DITHER                     = 1 << 2;
        const DEPTH_PREPASS                     = 1 << 3;
        const NORMAL_PREPASS                    = 1 << 4;
        const DEFERRED_PREPASS                  = 1 << 5;
        const MOTION_VECTOR_PREPASS             = 1 << 6;
        const MAY_DISCARD                       = 1 << 7; // Guards shader codepaths that may discard, allowing early depth tests in most cases
                                                            // See: https://www.khronos.org/opengl/wiki/Early_Fragment_Test
        const ENVIRONMENT_MAP                   = 1 << 8;
        const SCREEN_SPACE_AMBIENT_OCCLUSION    = 1 << 9;
        const UNCLIPPED_DEPTH_ORTHO             = 1 << 10; // Disables depth clipping for use with directional light shadow views
                                                            // Emulated via fragment shader depth on hardware that doesn't support it natively
                                                            // See: https://www.w3.org/TR/webgpu/#depth-clipping and https://therealmjp.github.io/posts/shadow-maps/#disabling-z-clipping
        const TEMPORAL_JITTER                   = 1 << 11;
        const READS_VIEW_TRANSMISSION_TEXTURE   = 1 << 12;
        const LIGHTMAPPED                       = 1 << 13;
        const LIGHTMAP_BICUBIC_SAMPLING         = 1 << 14;
        const IRRADIANCE_VOLUME                 = 1 << 15;
        const VISIBILITY_RANGE_DITHER           = 1 << 16;
        const SCREEN_SPACE_REFLECTIONS          = 1 << 17;
        const HAS_PREVIOUS_SKIN                 = 1 << 18;
        const HAS_PREVIOUS_MORPH                = 1 << 19;
        const OIT_ENABLED                       = 1 << 20;
        const DISTANCE_FOG                      = 1 << 21;
        const LAST_FLAG                         = Self::DISTANCE_FOG.bits();

        // Bitfields
        const MSAA_RESERVED_BITS                = Self::MSAA_MASK_BITS << Self::MSAA_SHIFT_BITS;
        const BLEND_RESERVED_BITS               = Self::BLEND_MASK_BITS << Self::BLEND_SHIFT_BITS; // ← Bitmask reserving bits for the blend state
        const BLEND_OPAQUE                      = 0 << Self::BLEND_SHIFT_BITS;                     // ← Values are just sequential within the mask
        const BLEND_PREMULTIPLIED_ALPHA         = 1 << Self::BLEND_SHIFT_BITS;                     // ← As blend states is on 3 bits, it can range from 0 to 7
        const BLEND_MULTIPLY                    = 2 << Self::BLEND_SHIFT_BITS;                     // ← See `BLEND_MASK_BITS` for the number of bits available
        const BLEND_ALPHA                       = 3 << Self::BLEND_SHIFT_BITS;                     //
        const BLEND_ALPHA_TO_COVERAGE           = 4 << Self::BLEND_SHIFT_BITS;                     // ← We still have room for three more values without adding more bits
        const TONEMAP_METHOD_RESERVED_BITS      = Self::TONEMAP_METHOD_MASK_BITS << Self::TONEMAP_METHOD_SHIFT_BITS;
        const TONEMAP_METHOD_NONE               = 0 << Self::TONEMAP_METHOD_SHIFT_BITS;
        const TONEMAP_METHOD_REINHARD           = 1 << Self::TONEMAP_METHOD_SHIFT_BITS;
        const TONEMAP_METHOD_REINHARD_LUMINANCE = 2 << Self::TONEMAP_METHOD_SHIFT_BITS;
        const TONEMAP_METHOD_ACES_FITTED        = 3 << Self::TONEMAP_METHOD_SHIFT_BITS;
        const TONEMAP_METHOD_AGX                = 4 << Self::TONEMAP_METHOD_SHIFT_BITS;
        const TONEMAP_METHOD_SOMEWHAT_BORING_DISPLAY_TRANSFORM = 5 << Self::TONEMAP_METHOD_SHIFT_BITS;
        const TONEMAP_METHOD_TONY_MC_MAPFACE    = 6 << Self::TONEMAP_METHOD_SHIFT_BITS;
        const TONEMAP_METHOD_BLENDER_FILMIC     = 7 << Self::TONEMAP_METHOD_SHIFT_BITS;
        const SHADOW_FILTER_METHOD_RESERVED_BITS = Self::SHADOW_FILTER_METHOD_MASK_BITS << Self::SHADOW_FILTER_METHOD_SHIFT_BITS;
        const SHADOW_FILTER_METHOD_HARDWARE_2X2  = 0 << Self::SHADOW_FILTER_METHOD_SHIFT_BITS;
        const SHADOW_FILTER_METHOD_GAUSSIAN      = 1 << Self::SHADOW_FILTER_METHOD_SHIFT_BITS;
        const SHADOW_FILTER_METHOD_TEMPORAL      = 2 << Self::SHADOW_FILTER_METHOD_SHIFT_BITS;
        const VIEW_PROJECTION_RESERVED_BITS     = Self::VIEW_PROJECTION_MASK_BITS << Self::VIEW_PROJECTION_SHIFT_BITS;
        const VIEW_PROJECTION_NONSTANDARD       = 0 << Self::VIEW_PROJECTION_SHIFT_BITS;
        const VIEW_PROJECTION_PERSPECTIVE       = 1 << Self::VIEW_PROJECTION_SHIFT_BITS;
        const VIEW_PROJECTION_ORTHOGRAPHIC      = 2 << Self::VIEW_PROJECTION_SHIFT_BITS;
        const VIEW_PROJECTION_RESERVED          = 3 << Self::VIEW_PROJECTION_SHIFT_BITS;
        const SCREEN_SPACE_SPECULAR_TRANSMISSION_RESERVED_BITS = Self::SCREEN_SPACE_SPECULAR_TRANSMISSION_MASK_BITS << Self::SCREEN_SPACE_SPECULAR_TRANSMISSION_SHIFT_BITS;
        const SCREEN_SPACE_SPECULAR_TRANSMISSION_LOW    = 0 << Self::SCREEN_SPACE_SPECULAR_TRANSMISSION_SHIFT_BITS;
        const SCREEN_SPACE_SPECULAR_TRANSMISSION_MEDIUM = 1 << Self::SCREEN_SPACE_SPECULAR_TRANSMISSION_SHIFT_BITS;
        const SCREEN_SPACE_SPECULAR_TRANSMISSION_HIGH   = 2 << Self::SCREEN_SPACE_SPECULAR_TRANSMISSION_SHIFT_BITS;
        const SCREEN_SPACE_SPECULAR_TRANSMISSION_ULTRA  = 3 << Self::SCREEN_SPACE_SPECULAR_TRANSMISSION_SHIFT_BITS;
        const ALL_RESERVED_BITS =
            Self::BLEND_RESERVED_BITS.bits() |
            Self::MSAA_RESERVED_BITS.bits() |
            Self::TONEMAP_METHOD_RESERVED_BITS.bits() |
            Self::SHADOW_FILTER_METHOD_RESERVED_BITS.bits() |
            Self::VIEW_PROJECTION_RESERVED_BITS.bits() |
            Self::SCREEN_SPACE_SPECULAR_TRANSMISSION_RESERVED_BITS.bits();
    }
}

impl MeshPipelineKey {
    const MSAA_MASK_BITS: u64 = 0b111;
    const MSAA_SHIFT_BITS: u64 = Self::LAST_FLAG.bits().trailing_zeros() as u64 + 1;

    const BLEND_MASK_BITS: u64 = 0b111;
    const BLEND_SHIFT_BITS: u64 = Self::MSAA_MASK_BITS.count_ones() as u64 + Self::MSAA_SHIFT_BITS;

    const TONEMAP_METHOD_MASK_BITS: u64 = 0b111;
    const TONEMAP_METHOD_SHIFT_BITS: u64 =
        Self::BLEND_MASK_BITS.count_ones() as u64 + Self::BLEND_SHIFT_BITS;

    const SHADOW_FILTER_METHOD_MASK_BITS: u64 = 0b11;
    const SHADOW_FILTER_METHOD_SHIFT_BITS: u64 =
        Self::TONEMAP_METHOD_MASK_BITS.count_ones() as u64 + Self::TONEMAP_METHOD_SHIFT_BITS;

    const VIEW_PROJECTION_MASK_BITS: u64 = 0b11;
    const VIEW_PROJECTION_SHIFT_BITS: u64 = Self::SHADOW_FILTER_METHOD_MASK_BITS.count_ones()
        as u64
        + Self::SHADOW_FILTER_METHOD_SHIFT_BITS;

    const SCREEN_SPACE_SPECULAR_TRANSMISSION_MASK_BITS: u64 = 0b11;
    const SCREEN_SPACE_SPECULAR_TRANSMISSION_SHIFT_BITS: u64 =
        Self::VIEW_PROJECTION_MASK_BITS.count_ones() as u64 + Self::VIEW_PROJECTION_SHIFT_BITS;

    pub fn from_msaa_samples(msaa_samples: u32) -> Self {
        let msaa_bits =
            (msaa_samples.trailing_zeros() as u64 & Self::MSAA_MASK_BITS) << Self::MSAA_SHIFT_BITS;
        Self::from_bits_retain(msaa_bits)
    }

    pub fn from_hdr(hdr: bool) -> Self {
        if hdr {
            MeshPipelineKey::HDR
        } else {
            MeshPipelineKey::NONE
        }
    }

    pub fn msaa_samples(&self) -> u32 {
        1 << ((self.bits() >> Self::MSAA_SHIFT_BITS) & Self::MSAA_MASK_BITS)
    }

    pub fn from_primitive_topology(primitive_topology: PrimitiveTopology) -> Self {
        let primitive_topology_bits = ((primitive_topology as u64)
            & BaseMeshPipelineKey::PRIMITIVE_TOPOLOGY_MASK_BITS)
            << BaseMeshPipelineKey::PRIMITIVE_TOPOLOGY_SHIFT_BITS;
        Self::from_bits_retain(primitive_topology_bits)
    }

    pub fn primitive_topology(&self) -> PrimitiveTopology {
        let primitive_topology_bits = (self.bits()
            >> BaseMeshPipelineKey::PRIMITIVE_TOPOLOGY_SHIFT_BITS)
            & BaseMeshPipelineKey::PRIMITIVE_TOPOLOGY_MASK_BITS;
        match primitive_topology_bits {
            x if x == PrimitiveTopology::PointList as u64 => PrimitiveTopology::PointList,
            x if x == PrimitiveTopology::LineList as u64 => PrimitiveTopology::LineList,
            x if x == PrimitiveTopology::LineStrip as u64 => PrimitiveTopology::LineStrip,
            x if x == PrimitiveTopology::TriangleList as u64 => PrimitiveTopology::TriangleList,
            x if x == PrimitiveTopology::TriangleStrip as u64 => PrimitiveTopology::TriangleStrip,
            _ => PrimitiveTopology::default(),
        }
    }
}

// Ensure that we didn't overflow the number of bits available in `MeshPipelineKey`.
const_assert_eq!(
    (((MeshPipelineKey::LAST_FLAG.bits() << 1) - 1) | MeshPipelineKey::ALL_RESERVED_BITS.bits())
        & BaseMeshPipelineKey::all().bits(),
    0
);

// Ensure that the reserved bits don't overlap with the topology bits
const_assert_eq!(
    (BaseMeshPipelineKey::PRIMITIVE_TOPOLOGY_MASK_BITS
        << BaseMeshPipelineKey::PRIMITIVE_TOPOLOGY_SHIFT_BITS)
        & MeshPipelineKey::ALL_RESERVED_BITS.bits(),
    0
);
