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
    render_resource::{CachedRenderPipelineId, TextureFormat},
    renderer::RenderDevice,
    settings::WgpuLimits,
};

pub const DEFERRED_PREPASS_FORMAT: TextureFormat = TextureFormat::Rgba32Uint;
pub const DEFERRED_LIGHTING_PASS_ID_FORMAT: TextureFormat = TextureFormat::R8Uint;
pub const DEFERRED_SPECULAR_TINT_FORMAT: TextureFormat = TextureFormat::R32Uint;

/// Whether the deferred pass has the specular tint target, for each combination of normal and
/// motion vector prepasses.
///
/// The tint target is supported when all deferred pass color targets fit in the device's
/// `max_color_attachments` and `max_color_attachment_bytes_per_sample` limits, and
/// `max_sampled_textures_per_shader_stage` is above the WebGPU minimum of 16. It is never supported
/// on WebGL2.
#[derive(Resource, Clone, Copy, Debug)]
pub struct DeferredSpecularTintSupport {
    /// Indexed by `normal_prepass as usize | (motion_vector_prepass as usize) << 1`.
    supported: [bool; 4],
}

impl DeferredSpecularTintSupport {
    /// Computes the support for every prepass combination from the device `limits`.
    pub fn new(limits: &WgpuLimits) -> Self {
        Self {
            supported: core::array::from_fn(|i| {
                deferred_specular_tint_fits(limits, i & 1 != 0, i & 2 != 0)
            }),
        }
    }

    /// Returns whether the deferred pass of a view with these prepasses has the specular tint
    /// target.
    pub fn is_supported(&self, normal_prepass: bool, motion_vector_prepass: bool) -> bool {
        self.supported[normal_prepass as usize | ((motion_vector_prepass as usize) << 1)]
    }
}

/// Inserts [`DeferredSpecularTintSupport`] for the limits of the [`RenderDevice`].
pub fn init_deferred_specular_tint_support(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
) {
    commands.insert_resource(DeferredSpecularTintSupport::new(&render_device.limits()));
}

fn deferred_specular_tint_fits(
    limits: &WgpuLimits,
    normal_prepass: bool,
    motion_vector_prepass: bool,
) -> bool {
    // WebGL2 can't clear integer attachments with a load operation.
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
    // The mesh view and material bind groups can already use 16 sampled textures in the fragment
    // stage, and the tint texture adds one to the mesh view bind group.
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
