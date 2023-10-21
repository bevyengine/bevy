mod camera_2d;
mod main_pass_2d_node;

pub mod graph {
    pub const NAME: &str = "core_2d";
    pub mod input {
        pub const VIEW_ENTITY: &str = "view_entity";
    }
    pub mod node {
        pub const MSAA_WRITEBACK: &str = "msaa_writeback";
        pub const MAIN_PASS: &str = "main_pass";
        pub const BLOOM: &str = "bloom";
        pub const TONEMAPPING: &str = "tonemapping";
        pub const FXAA: &str = "fxaa";
        pub const UPSCALING: &str = "upscaling";
        pub const CONTRAST_ADAPTIVE_SHARPENING: &str = "contrast_adaptive_sharpening";
        pub const END_MAIN_PASS_POST_PROCESSING: &str = "end_main_pass_post_processing";
    }
}
pub const CORE_2D: &str = graph::NAME;

use std::ops::Range;

pub use camera_2d::*;
pub use main_pass_2d_node::*;

use bevy_app::{App, Plugin};
use bevy_ecs::prelude::*;
use bevy_render::{
    camera::Camera,
    extract_component::ExtractComponentPlugin,
    render_graph::{EmptyNode, RenderGraphApp, ViewNodeRunner},
    render_phase::{
        sort_phase_system, CachedRenderPipelinePhaseItem, DrawFunctionId, DrawFunctions, PhaseItem,
        RenderPhase,
    },
    render_resource::CachedRenderPipelineId,
    Extract, ExtractSchedule, Render, RenderApp, RenderSet,
};
use bevy_utils::{nonmax::NonMaxU32, FloatOrd};

use crate::{tonemapping::TonemappingNode, upscaling::UpscalingNode};

pub struct Core2dPlugin;

impl Plugin for Core2dPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Camera2d>()
            .add_plugins(ExtractComponentPlugin::<Camera2d>::default());

        let render_app = match app.get_sub_app_mut(RenderApp) {
            Ok(render_app) => render_app,
            Err(_) => return,
        };

        render_app
            .init_resource::<DrawFunctions<Transparent2d>>()
            .add_systems(ExtractSchedule, extract_core_2d_camera_phases)
            .add_systems(
                Render,
                sort_phase_system::<Transparent2d>.in_set(RenderSet::PhaseSort),
            );

        use graph::node::*;
        render_app
            .add_render_sub_graph(CORE_2D)
            .add_render_graph_node::<MainPass2dNode>(CORE_2D, MAIN_PASS)
            .add_render_graph_node::<ViewNodeRunner<TonemappingNode>>(CORE_2D, TONEMAPPING)
            .add_render_graph_node::<EmptyNode>(CORE_2D, END_MAIN_PASS_POST_PROCESSING)
            .add_render_graph_node::<ViewNodeRunner<UpscalingNode>>(CORE_2D, UPSCALING)
            .add_render_graph_edges(
                CORE_2D,
                &[
                    MAIN_PASS,
                    TONEMAPPING,
                    END_MAIN_PASS_POST_PROCESSING,
                    UPSCALING,
                ],
            );
    }
}

pub struct Transparent2d {
    pub sort_key: FloatOrd,
    pub entity: Entity,
    pub pipeline: CachedRenderPipelineId,
    pub draw_function: DrawFunctionId,
    pub batch_range: Range<u32>,
    pub dynamic_offset: Option<NonMaxU32>,
}

impl PhaseItem for Transparent2d {
    type SortKey = FloatOrd;

    #[inline]
    fn entity(&self) -> Entity {
        self.entity
    }

    #[inline]
    fn sort_key(&self) -> Self::SortKey {
        self.sort_key
    }

    #[inline]
    fn draw_function(&self) -> DrawFunctionId {
        self.draw_function
    }

    #[inline]
    fn sort(items: &mut [Self]) {
        // radsort is a stable radix sort that performed better than `slice::sort_by_key` or `slice::sort_unstable_by_key`.
        radsort::sort_by_key(items, |item| item.sort_key().0);
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
    fn dynamic_offset(&self) -> Option<NonMaxU32> {
        self.dynamic_offset
    }

    #[inline]
    fn dynamic_offset_mut(&mut self) -> &mut Option<NonMaxU32> {
        &mut self.dynamic_offset
    }
}

impl CachedRenderPipelinePhaseItem for Transparent2d {
    #[inline]
    fn cached_pipeline(&self) -> CachedRenderPipelineId {
        self.pipeline
    }
}

pub fn extract_core_2d_camera_phases(
    mut commands: Commands,
    cameras_2d: Extract<Query<(Entity, &Camera), With<Camera2d>>>,
) {
    for (entity, camera) in &cameras_2d {
        if camera.is_active {
            commands
                .get_or_spawn(entity)
                .insert(RenderPhase::<Transparent2d>::default());
        }
    }
}
