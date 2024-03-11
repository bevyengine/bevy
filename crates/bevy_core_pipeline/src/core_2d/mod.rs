mod camera_2d;
mod main_pass_2d_node;

pub mod graph {
    use bevy_render::render_graph::{RenderLabel, RenderSubGraph};

    #[derive(Debug, Hash, PartialEq, Eq, Clone, RenderSubGraph)]
    pub struct Core2d;

    pub mod input {
        pub const VIEW_ENTITY: &str = "view_entity";
    }

    #[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
    pub enum Node2d {
        MsaaWriteback,
        MainPass,
        Bloom,
        Tonemapping,
        Fxaa,
        Upscaling,
        ContrastAdaptiveSharpening,
        EndMainPassPostProcessing,
    }
}

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
use bevy_utils::FloatOrd;
use nonmax::NonMaxU32;

use crate::{tonemapping::TonemappingNode, upscaling::UpscalingNode};

use self::graph::{Core2d, Node2d};

pub struct Core2dPlugin;

impl Plugin for Core2dPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Camera2d>()
            .add_plugins(ExtractComponentPlugin::<Camera2d>::default());

        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .init_resource::<DrawFunctions<Transparent2d>>()
            .add_systems(ExtractSchedule, extract_core_2d_camera_phases)
            .add_systems(
                Render,
                sort_phase_system::<Transparent2d>.in_set(RenderSet::PhaseSort),
            );

        render_app
            .add_render_sub_graph(Core2d)
            .add_render_graph_node::<MainPass2dNode>(Core2d, Node2d::MainPass)
            .add_render_graph_node::<ViewNodeRunner<TonemappingNode>>(Core2d, Node2d::Tonemapping)
            .add_render_graph_node::<EmptyNode>(Core2d, Node2d::EndMainPassPostProcessing)
            .add_render_graph_node::<ViewNodeRunner<UpscalingNode>>(Core2d, Node2d::Upscaling)
            .add_render_graph_edges(
                Core2d,
                (
                    Node2d::MainPass,
                    Node2d::Tonemapping,
                    Node2d::EndMainPassPostProcessing,
                    Node2d::Upscaling,
                ),
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
