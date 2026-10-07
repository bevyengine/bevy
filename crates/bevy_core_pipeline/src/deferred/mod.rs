pub mod copy_lighting_id;
pub mod node;

use core::ops::Range;

use crate::prepass::{
    OpaqueNoLightmap3dBatchSetKey, OpaqueNoLightmap3dBinKey, MOTION_VECTOR_PREPASS_FORMAT,
    NORMAL_PREPASS_FORMAT,
};
use bevy_ecs::prelude::*;
use bevy_render::sync_world::MainEntity;
use bevy_render::{
    render_phase::{
        BinnedPhaseItem, CachedRenderPipelinePhaseItem, DrawFunctionId, PhaseItem,
        PhaseItemExtraIndex,
    },
    render_resource::{
        CachedRenderPipelineId, Extent3d, TextureDataOrder, TextureDescriptor, TextureDimension,
        TextureFormat, TextureUsages, TextureView, TextureViewDescriptor,
    },
    renderer::{RenderDevice, RenderQueue},
    settings::WgpuLimits,
};

pub const DEFERRED_PREPASS_FORMAT: TextureFormat = TextureFormat::Rgba32Uint;
pub const DEFERRED_LIGHTING_PASS_ID_FORMAT: TextureFormat = TextureFormat::R8Uint;
/// The format of the deferred specular tint texture.
///
/// Each texel holds the specular tint in rgb9e5, combined by exclusive or with the rgb9e5 encoding
/// of white, so that a value of 0 decodes to white. The deferred pass has this target only when
/// [`deferred_specular_tint_fits`] returns `true`.
pub const DEFERRED_SPECULAR_TINT_FORMAT: TextureFormat = TextureFormat::R32Uint;

/// Returns whether the deferred pass can add the specular tint target to its other color
/// attachments within `limits`.
///
/// The deferred pass writes the normal and motion vector targets when their prepasses are enabled,
/// then the G-buffer, the lighting pass ID and the specular tint. This function sums the byte cost
/// of these targets with the same alignment rules that wgpu uses, and compares the sum with
/// `max_color_attachment_bytes_per_sample`. It also checks `max_color_attachments`, and requires
/// `max_sampled_textures_per_shader_stage` above the WebGPU minimum of 16, because the tint adds a
/// sampled texture to the mesh view bind group. On WebGL2 it always returns `false`.
pub fn deferred_specular_tint_fits(
    limits: &WgpuLimits,
    normal_prepass: bool,
    motion_vector_prepass: bool,
) -> bool {
    // WebGL2 can't clear integer attachments with a load operation, and its mesh view bind group
    // has no binding for the specular tint.
    if cfg!(all(
        feature = "webgl",
        target_arch = "wasm32",
        not(feature = "webgpu")
    )) {
        return false;
    }

    let formats = [
        normal_prepass.then_some(NORMAL_PREPASS_FORMAT),
        motion_vector_prepass.then_some(MOTION_VECTOR_PREPASS_FORMAT),
        Some(DEFERRED_PREPASS_FORMAT),
        Some(DEFERRED_LIGHTING_PASS_ID_FORMAT),
        Some(DEFERRED_SPECULAR_TINT_FORMAT),
    ];
    // Pipelines that use the mesh view bind group and a material bind group can already use 16
    // sampled textures in the fragment stage.
    if limits.max_sampled_textures_per_shader_stage <= 16 {
        return false;
    }
    if formats.len() as u32 > limits.max_color_attachments {
        return false;
    }
    let mut bytes_per_sample = 0u32;
    for format in formats.into_iter().flatten() {
        let (Some(cost), Some(alignment)) = (
            format.target_pixel_byte_cost(),
            format.target_component_alignment(),
        ) else {
            return false;
        };
        bytes_per_sample = bytes_per_sample.next_multiple_of(alignment) + cost;
    }
    bytes_per_sample <= limits.max_color_attachment_bytes_per_sample
}

/// A 1x1 [`DEFERRED_SPECULAR_TINT_FORMAT`] texture that holds 0, which decodes to a white
/// specular tint.
///
/// Passes that always bind a specular tint texture, such as Solari, bind it when the view has no
/// deferred specular tint texture.
#[derive(Resource)]
pub struct DeferredSpecularTintFallback {
    pub view: TextureView,
}

pub fn init_deferred_specular_tint_fallback(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
) {
    let texture = render_device.create_texture_with_data(
        &render_queue,
        &TextureDescriptor {
            label: Some("deferred_specular_tint_fallback"),
            size: Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: DEFERRED_SPECULAR_TINT_FORMAT,
            usage: TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        },
        TextureDataOrder::default(),
        &[0; 4],
    );
    commands.insert_resource(DeferredSpecularTintFallback {
        view: texture.create_view(&TextureViewDescriptor::default()),
    });
}
pub const DEFERRED_LIGHTING_PASS_ID_DEPTH_FORMAT: TextureFormat = TextureFormat::Depth16Unorm;

/// Opaque phase of the 3D Deferred pass.
///
/// Sorted by pipeline, then by mesh to improve batching.
///
/// Used to render all 3D meshes with materials that have no transparency.
#[derive(PartialEq, Eq, Hash)]
pub struct Opaque3dDeferred {
    /// Determines which objects can be placed into a *batch set*.
    ///
    /// Objects in a single batch set can potentially be multi-drawn together,
    /// if it's enabled and the current platform supports it.
    pub batch_set_key: OpaqueNoLightmap3dBatchSetKey,
    /// Information that separates items into bins.
    pub bin_key: OpaqueNoLightmap3dBinKey,
    pub representative_entity: (Entity, MainEntity),
    pub batch_range: Range<u32>,
    pub extra_index: PhaseItemExtraIndex,
}

impl PhaseItem for Opaque3dDeferred {
    #[inline]
    fn entity(&self) -> Entity {
        self.representative_entity.0
    }

    fn main_entity(&self) -> MainEntity {
        self.representative_entity.1
    }

    #[inline]
    fn draw_function(&self) -> DrawFunctionId {
        self.batch_set_key.draw_function
    }

    #[inline]
    fn batch_range(&self) -> &Range<u32> {
        &self.batch_range
    }

    #[inline]
    fn batch_range_mut(&mut self) -> &mut Range<u32> {
        &mut self.batch_range
    }

    #[inline]
    fn extra_index(&self) -> PhaseItemExtraIndex {
        self.extra_index.clone()
    }

    #[inline]
    fn batch_range_and_extra_index_mut(&mut self) -> (&mut Range<u32>, &mut PhaseItemExtraIndex) {
        (&mut self.batch_range, &mut self.extra_index)
    }
}

impl BinnedPhaseItem for Opaque3dDeferred {
    type BatchSetKey = OpaqueNoLightmap3dBatchSetKey;
    type BinKey = OpaqueNoLightmap3dBinKey;

    #[inline]
    fn new(
        batch_set_key: Self::BatchSetKey,
        bin_key: Self::BinKey,
        representative_entity: (Entity, MainEntity),
        batch_range: Range<u32>,
        extra_index: PhaseItemExtraIndex,
    ) -> Self {
        Self {
            batch_set_key,
            bin_key,
            representative_entity,
            batch_range,
            extra_index,
        }
    }
}

impl CachedRenderPipelinePhaseItem for Opaque3dDeferred {
    #[inline]
    fn cached_pipeline(&self) -> CachedRenderPipelineId {
        self.batch_set_key.pipeline
    }
}

/// Alpha mask phase of the 3D Deferred pass.
///
/// Sorted by pipeline, then by mesh to improve batching.
///
/// Used to render all meshes with a material with an alpha mask.
pub struct AlphaMask3dDeferred {
    /// Determines which objects can be placed into a *batch set*.
    ///
    /// Objects in a single batch set can potentially be multi-drawn together,
    /// if it's enabled and the current platform supports it.
    pub batch_set_key: OpaqueNoLightmap3dBatchSetKey,
    /// Information that separates items into bins.
    pub bin_key: OpaqueNoLightmap3dBinKey,
    pub representative_entity: (Entity, MainEntity),
    pub batch_range: Range<u32>,
    pub extra_index: PhaseItemExtraIndex,
}

impl PhaseItem for AlphaMask3dDeferred {
    #[inline]
    fn entity(&self) -> Entity {
        self.representative_entity.0
    }

    #[inline]
    fn main_entity(&self) -> MainEntity {
        self.representative_entity.1
    }

    #[inline]
    fn draw_function(&self) -> DrawFunctionId {
        self.batch_set_key.draw_function
    }

    #[inline]
    fn batch_range(&self) -> &Range<u32> {
        &self.batch_range
    }

    #[inline]
    fn batch_range_mut(&mut self) -> &mut Range<u32> {
        &mut self.batch_range
    }

    #[inline]
    fn extra_index(&self) -> PhaseItemExtraIndex {
        self.extra_index.clone()
    }

    #[inline]
    fn batch_range_and_extra_index_mut(&mut self) -> (&mut Range<u32>, &mut PhaseItemExtraIndex) {
        (&mut self.batch_range, &mut self.extra_index)
    }
}

impl BinnedPhaseItem for AlphaMask3dDeferred {
    type BatchSetKey = OpaqueNoLightmap3dBatchSetKey;
    type BinKey = OpaqueNoLightmap3dBinKey;

    fn new(
        batch_set_key: Self::BatchSetKey,
        bin_key: Self::BinKey,
        representative_entity: (Entity, MainEntity),
        batch_range: Range<u32>,
        extra_index: PhaseItemExtraIndex,
    ) -> Self {
        Self {
            batch_set_key,
            bin_key,
            representative_entity,
            batch_range,
            extra_index,
        }
    }
}

impl CachedRenderPipelinePhaseItem for AlphaMask3dDeferred {
    #[inline]
    fn cached_pipeline(&self) -> CachedRenderPipelineId {
        self.batch_set_key.pipeline
    }
}
