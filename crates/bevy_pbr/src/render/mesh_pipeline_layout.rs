use std::array;

use bevy_core_pipeline::{
    prepass::ViewPrepassTextures, tonemapping::get_lut_bind_group_layout_entries,
};
use bevy_render::{
    globals::GlobalsUniform,
    render_resource::{
        BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType,
        BufferBindingType, SamplerBindingType, ShaderStages, ShaderType, TextureSampleType,
        TextureViewDimension,
    },
    renderer::RenderDevice,
    view::{Msaa, ViewUniform},
};

use crate::{
    environment_map, prepass, GpuFog, GpuLights, GpuPointLights, MeshPipelineKey,
    ViewClusterBindings,
};

#[derive(Clone)]
pub struct MeshPipelineViewLayout {
    pub bind_group_layout: BindGroupLayout,

    #[cfg(debug_assertions)]
    pub texture_count: usize,
}

bitflags::bitflags! {
    /// A key that uniquely identifies a [`MeshPipelineViewLayout`].
    ///
    /// Used to generate all possible layouts for the mesh pipeline in [`generate_view_layouts`],
    /// so special care must be taken to not add too many flags, as the number of possible layouts
    /// will grow exponentially.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    #[repr(transparent)]
    pub struct MeshPipelineViewLayoutKey: u32 {
        const MULTISAMPLED                = (1 << 0);
        const DEPTH_PREPASS               = (1 << 1);
        const NORMAL_PREPASS              = (1 << 2);
        const MOTION_VECTOR_PREPASS       = (1 << 3);
        const DEFERRED_PREPASS            = (1 << 4);
    }
}

impl MeshPipelineViewLayoutKey {
    // The number of possible layouts
    pub const COUNT: usize = Self::all().bits() as usize + 1;

    /// Builds a unique label for each layout based on the flags
    pub fn label(&self) -> String {
        use MeshPipelineViewLayoutKey as Key;

        format!(
            "mesh_view_layout{}{}{}{}{}",
            self.contains(Key::MULTISAMPLED)
                .then_some("_multisampled")
                .unwrap_or_default(),
            self.contains(Key::DEPTH_PREPASS)
                .then_some("_depth")
                .unwrap_or_default(),
            self.contains(Key::NORMAL_PREPASS)
                .then_some("_normal")
                .unwrap_or_default(),
            self.contains(Key::MOTION_VECTOR_PREPASS)
                .then_some("_motion")
                .unwrap_or_default(),
            self.contains(Key::DEFERRED_PREPASS)
                .then_some("_deferred")
                .unwrap_or_default(),
        )
    }
}

impl From<MeshPipelineKey> for MeshPipelineViewLayoutKey {
    fn from(value: MeshPipelineKey) -> Self {
        let mut result = MeshPipelineViewLayoutKey::empty();

        if value.msaa_samples() > 1 {
            result |= MeshPipelineViewLayoutKey::MULTISAMPLED;
        }
        if value.contains(MeshPipelineKey::DEPTH_PREPASS) {
            result |= MeshPipelineViewLayoutKey::DEPTH_PREPASS;
        }
        if value.contains(MeshPipelineKey::NORMAL_PREPASS) {
            result |= MeshPipelineViewLayoutKey::NORMAL_PREPASS;
        }
        if value.contains(MeshPipelineKey::MOTION_VECTOR_PREPASS) {
            result |= MeshPipelineViewLayoutKey::MOTION_VECTOR_PREPASS;
        }
        if value.contains(MeshPipelineKey::DEFERRED_PREPASS) {
            result |= MeshPipelineViewLayoutKey::DEFERRED_PREPASS;
        }

        result
    }
}

impl From<Msaa> for MeshPipelineViewLayoutKey {
    fn from(value: Msaa) -> Self {
        let mut result = MeshPipelineViewLayoutKey::empty();

        if value.samples() > 1 {
            result |= MeshPipelineViewLayoutKey::MULTISAMPLED;
        }

        result
    }
}

impl From<Option<&ViewPrepassTextures>> for MeshPipelineViewLayoutKey {
    fn from(value: Option<&ViewPrepassTextures>) -> Self {
        let mut result = MeshPipelineViewLayoutKey::empty();

        if let Some(prepass_textures) = value {
            if prepass_textures.depth.is_some() {
                result |= MeshPipelineViewLayoutKey::DEPTH_PREPASS;
            }
            if prepass_textures.normal.is_some() {
                result |= MeshPipelineViewLayoutKey::NORMAL_PREPASS;
            }
            if prepass_textures.motion_vectors.is_some() {
                result |= MeshPipelineViewLayoutKey::MOTION_VECTOR_PREPASS;
            }
            if prepass_textures.deferred.is_some() {
                result |= MeshPipelineViewLayoutKey::DEFERRED_PREPASS;
            }
        }

        result
    }
}

/// Returns the appropriate bind group layout vec based on the parameters
fn layout_entries(
    clustered_forward_buffer_binding_type: BufferBindingType,
    layout_key: MeshPipelineViewLayoutKey,
) -> Vec<BindGroupLayoutEntry> {
    let mut entries = vec![
        // View
        BindGroupLayoutEntry {
            binding: 0,
            visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Uniform,
                has_dynamic_offset: true,
                min_binding_size: Some(ViewUniform::min_size()),
            },
            count: None,
        },
        // Lights
        BindGroupLayoutEntry {
            binding: 1,
            visibility: ShaderStages::FRAGMENT,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Uniform,
                has_dynamic_offset: true,
                min_binding_size: Some(GpuLights::min_size()),
            },
            count: None,
        },
        // Point Shadow Texture Cube Array
        BindGroupLayoutEntry {
            binding: 2,
            visibility: ShaderStages::FRAGMENT,
            ty: BindingType::Texture {
                multisampled: false,
                sample_type: TextureSampleType::Depth,
                #[cfg(any(not(feature = "webgl"), not(target_arch = "wasm32")))]
                view_dimension: TextureViewDimension::CubeArray,
                #[cfg(all(feature = "webgl", target_arch = "wasm32"))]
                view_dimension: TextureViewDimension::Cube,
            },
            count: None,
        },
        // Point Shadow Texture Array Sampler
        BindGroupLayoutEntry {
            binding: 3,
            visibility: ShaderStages::FRAGMENT,
            ty: BindingType::Sampler(SamplerBindingType::Comparison),
            count: None,
        },
        // Directional Shadow Texture Array
        BindGroupLayoutEntry {
            binding: 4,
            visibility: ShaderStages::FRAGMENT,
            ty: BindingType::Texture {
                multisampled: false,
                sample_type: TextureSampleType::Depth,
                #[cfg(any(not(feature = "webgl"), not(target_arch = "wasm32")))]
                view_dimension: TextureViewDimension::D2Array,
                #[cfg(all(feature = "webgl", target_arch = "wasm32"))]
                view_dimension: TextureViewDimension::D2,
            },
            count: None,
        },
        // Directional Shadow Texture Array Sampler
        BindGroupLayoutEntry {
            binding: 5,
            visibility: ShaderStages::FRAGMENT,
            ty: BindingType::Sampler(SamplerBindingType::Comparison),
            count: None,
        },
        // PointLights
        BindGroupLayoutEntry {
            binding: 6,
            visibility: ShaderStages::FRAGMENT,
            ty: BindingType::Buffer {
                ty: clustered_forward_buffer_binding_type,
                has_dynamic_offset: false,
                min_binding_size: Some(GpuPointLights::min_size(
                    clustered_forward_buffer_binding_type,
                )),
            },
            count: None,
        },
        // ClusteredLightIndexLists
        BindGroupLayoutEntry {
            binding: 7,
            visibility: ShaderStages::FRAGMENT,
            ty: BindingType::Buffer {
                ty: clustered_forward_buffer_binding_type,
                has_dynamic_offset: false,
                min_binding_size: Some(ViewClusterBindings::min_size_cluster_light_index_lists(
                    clustered_forward_buffer_binding_type,
                )),
            },
            count: None,
        },
        // ClusterOffsetsAndCounts
        BindGroupLayoutEntry {
            binding: 8,
            visibility: ShaderStages::FRAGMENT,
            ty: BindingType::Buffer {
                ty: clustered_forward_buffer_binding_type,
                has_dynamic_offset: false,
                min_binding_size: Some(ViewClusterBindings::min_size_cluster_offsets_and_counts(
                    clustered_forward_buffer_binding_type,
                )),
            },
            count: None,
        },
        // Globals
        BindGroupLayoutEntry {
            binding: 9,
            visibility: ShaderStages::VERTEX_FRAGMENT,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: Some(GlobalsUniform::min_size()),
            },
            count: None,
        },
        // Fog
        BindGroupLayoutEntry {
            binding: 10,
            visibility: ShaderStages::FRAGMENT,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Uniform,
                has_dynamic_offset: true,
                min_binding_size: Some(GpuFog::min_size()),
            },
            count: None,
        },
        // Screen space ambient occlusion texture
        BindGroupLayoutEntry {
            binding: 11,
            visibility: ShaderStages::FRAGMENT,
            ty: BindingType::Texture {
                multisampled: false,
                sample_type: TextureSampleType::Float { filterable: false },
                view_dimension: TextureViewDimension::D2,
            },
            count: None,
        },
    ];

    // EnvironmentMapLight
    let environment_map_entries = environment_map::get_bind_group_layout_entries([12, 13, 14]);
    entries.extend_from_slice(&environment_map_entries);

    // Tonemapping
    let tonemapping_lut_entries = get_lut_bind_group_layout_entries([15, 16]);
    entries.extend_from_slice(&tonemapping_lut_entries);

    if cfg!(any(not(feature = "webgl"), not(target_arch = "wasm32")))
        || (cfg!(all(feature = "webgl", target_arch = "wasm32"))
            && !layout_key.contains(MeshPipelineViewLayoutKey::MULTISAMPLED))
    {
        entries.extend_from_slice(&prepass::get_bind_group_layout_entries(
            [17, 18, 19, 20],
            layout_key,
        ));
    }

    entries
}

/// Generates all possible view layouts for the mesh pipeline, based on all combinations of
/// [`MeshPipelineViewLayoutKey`] flags.
pub fn generate_view_layouts(
    render_device: &RenderDevice,
    clustered_forward_buffer_binding_type: BufferBindingType,
) -> [MeshPipelineViewLayout; MeshPipelineViewLayoutKey::COUNT] {
    array::from_fn(|i| {
        let key = MeshPipelineViewLayoutKey::from_bits_truncate(i as u32);
        let entries = layout_entries(clustered_forward_buffer_binding_type, key);

        #[cfg(debug_assertions)]
        let texture_count: usize = entries
            .iter()
            .filter(|entry| matches!(entry.ty, BindingType::Texture { .. }))
            .count();

        MeshPipelineViewLayout {
            bind_group_layout: render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(key.label().as_str()),
                entries: &entries,
            }),
            #[cfg(debug_assertions)]
            texture_count,
        }
    })
}
