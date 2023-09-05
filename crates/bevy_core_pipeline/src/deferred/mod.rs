pub mod copy_lighting_id;
pub mod node;

use std::cmp::Reverse;

use bevy_ecs::prelude::*;
use bevy_render::{
    render_phase::{CachedRenderPipelinePhaseItem, DrawFunctionId, PhaseItem},
    render_resource::{CachedRenderPipelineId, TextureFormat},
};
use bevy_utils::FloatOrd;

pub const DEFERRED_PREPASS_FORMAT: TextureFormat = TextureFormat::Rgba32Uint;
pub const DEFERRED_LIGHTING_PASS_ID_FORMAT: TextureFormat = TextureFormat::R8Uint;
pub const DEFERRED_LIGHTING_PASS_ID_DEPTH_FORMAT: TextureFormat = TextureFormat::Depth32Float;

/// Opaque phase of the 3D Deferred pass.
///
/// Sorted front-to-back by the z-distance in front of the camera.
///
/// Used to render all 3D meshes with materials that have no transparency.
pub struct Opaque3dDeferred {
    pub distance: f32,
    pub entity: Entity,
    pub pipeline_id: CachedRenderPipelineId,
    pub draw_function: DrawFunctionId,
}

impl PhaseItem for Opaque3dDeferred {
    // NOTE: Values increase towards the camera. Front-to-back ordering for opaque means we need a descending sort.
    type SortKey = Reverse<FloatOrd>;

    #[inline]
    fn entity(&self) -> Entity {
        self.entity
    }

    #[inline]
    fn sort_key(&self) -> Self::SortKey {
        Reverse(FloatOrd(self.distance))
    }

    #[inline]
    fn draw_function(&self) -> DrawFunctionId {
        self.draw_function
    }

    #[inline]
    fn sort(items: &mut [Self]) {
        // Key negated to match reversed SortKey ordering
        radsort::sort_by_key(items, |item| -item.distance);
    }
}

impl CachedRenderPipelinePhaseItem for Opaque3dDeferred {
    #[inline]
    fn cached_pipeline(&self) -> CachedRenderPipelineId {
        self.pipeline_id
    }
}

/// Alpha mask phase of the 3D Deferred pass.
///
/// Sorted front-to-back by the z-distance in front of the camera.
///
/// Used to render all meshes with a material with an alpha mask.
pub struct AlphaMask3dDeferred {
    pub distance: f32,
    pub entity: Entity,
    pub pipeline_id: CachedRenderPipelineId,
    pub draw_function: DrawFunctionId,
}

impl PhaseItem for AlphaMask3dDeferred {
    // NOTE: Values increase towards the camera. Front-to-back ordering for alpha mask means we need a descending sort.
    type SortKey = Reverse<FloatOrd>;

    #[inline]
    fn entity(&self) -> Entity {
        self.entity
    }

    #[inline]
    fn sort_key(&self) -> Self::SortKey {
        Reverse(FloatOrd(self.distance))
    }

    #[inline]
    fn draw_function(&self) -> DrawFunctionId {
        self.draw_function
    }

    #[inline]
    fn sort(items: &mut [Self]) {
        // Key negated to match reversed SortKey ordering
        radsort::sort_by_key(items, |item| -item.distance);
    }
}

impl CachedRenderPipelinePhaseItem for AlphaMask3dDeferred {
    #[inline]
    fn cached_pipeline(&self) -> CachedRenderPipelineId {
        self.pipeline_id
    }
}
