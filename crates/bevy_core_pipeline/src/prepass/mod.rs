pub mod node;

use bevy_ecs::prelude::*;
use bevy_render::{
    render_phase::{CachedRenderPipelinePhaseItem, DrawFunctionId, EntityPhaseItem, PhaseItem},
    render_resource::{CachedRenderPipelineId, Extent3d},
    texture::CachedTexture,
};
use bevy_utils::FloatOrd;

/// Add a `PrepassSettings` component to a view to perform a depth and/or normal prepass.
/// These textures are useful for reducing overdraw in the main pass, and screen-space effects.
#[derive(Clone, Component)]
pub struct PrepassSettings {
    /// If true then depth values will be copied to a separate texture available to the main pass.
    pub output_depth: bool,
    /// If true then vertex world normals will be copied to a separate texture available to the main pass.
    pub output_normals: bool,
}

impl Default for PrepassSettings {
    fn default() -> Self {
        Self {
            output_depth: true,
            output_normals: true,
        }
    }
}

#[derive(Component)]
pub struct ViewPrepassTextures {
    pub depth: Option<CachedTexture>,
    pub normals: Option<CachedTexture>,
    pub size: Extent3d,
}

pub struct Opaque3dPrepass {
    pub distance: f32,
    pub entity: Entity,
    pub pipeline_id: CachedRenderPipelineId,
    pub draw_function: DrawFunctionId,
}

impl PhaseItem for Opaque3dPrepass {
    type SortKey = FloatOrd;

    #[inline]
    fn sort_key(&self) -> Self::SortKey {
        FloatOrd(self.distance)
    }

    #[inline]
    fn draw_function(&self) -> DrawFunctionId {
        self.draw_function
    }

    #[inline]
    fn sort(items: &mut [Self]) {
        radsort::sort_by_key(items, |item| item.distance);
    }
}

impl EntityPhaseItem for Opaque3dPrepass {
    fn entity(&self) -> Entity {
        self.entity
    }
}

impl CachedRenderPipelinePhaseItem for Opaque3dPrepass {
    #[inline]
    fn cached_pipeline(&self) -> CachedRenderPipelineId {
        self.pipeline_id
    }
}

pub struct AlphaMask3dPrepass {
    pub distance: f32,
    pub entity: Entity,
    pub pipeline_id: CachedRenderPipelineId,
    pub draw_function: DrawFunctionId,
}

impl PhaseItem for AlphaMask3dPrepass {
    type SortKey = FloatOrd;

    #[inline]
    fn sort_key(&self) -> Self::SortKey {
        FloatOrd(self.distance)
    }

    #[inline]
    fn draw_function(&self) -> DrawFunctionId {
        self.draw_function
    }

    #[inline]
    fn sort(items: &mut [Self]) {
        radsort::sort_by_key(items, |item| item.distance);
    }
}

impl EntityPhaseItem for AlphaMask3dPrepass {
    fn entity(&self) -> Entity {
        self.entity
    }
}

impl CachedRenderPipelinePhaseItem for AlphaMask3dPrepass {
    #[inline]
    fn cached_pipeline(&self) -> CachedRenderPipelineId {
        self.pipeline_id
    }
}
