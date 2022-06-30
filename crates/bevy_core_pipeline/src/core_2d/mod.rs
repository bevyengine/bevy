mod camera_2d;
mod main_pass_2d_node;

pub mod graph {
    pub const NAME: &str = "core_2d";
    pub mod input {
        pub const VIEW_ENTITY: &str = "view_entity";
    }
    pub mod node {
        pub const MAIN_PASS: &str = "main_pass";
    }
}

pub use camera_2d::*;
pub use main_pass_2d_node::*;

use bevy_app::{App, Plugin};
use bevy_ecs::prelude::*;
use bevy_render::{
    camera::Camera,
    extract_component::ExtractComponentPlugin,
    render_graph::{RenderGraph, SlotInfo, SlotType},
    render_phase::{
        batch_phase_system, sort_phase_system, BatchedPhaseItem, CachedRenderPipelinePhaseItem,
        DrawFunctionId, DrawFunctions, EntityPhaseItem, PhaseItem, RenderPhase,
    },
    render_resource::CachedRenderPipelineId,
    Extract, RenderApp, RenderStage,
};
use bevy_utils::FloatOrd;
use std::ops::Range;

pub struct Core2dPlugin;

impl Plugin for Core2dPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Camera2d>()
            .add_plugin(ExtractComponentPlugin::<Camera2d>::default());

        let render_app = match app.get_sub_app_mut(RenderApp) {
            Ok(render_app) => render_app,
            Err(_) => return,
        };

        render_app
            .init_resource::<DrawFunctions<Transparent2d>>()
            .add_system_to_stage(RenderStage::Extract, extract_core_2d_camera_phases)
            .add_system_to_stage(RenderStage::PhaseSort, sort_phase_system::<Transparent2d>)
            .add_system_to_stage(RenderStage::PhaseSort, batch_phase_system::<Transparent2d>);

        let pass_node_2d = MainPass2dNode::new(&mut render_app.world);
        let mut graph = render_app.world.resource_mut::<RenderGraph>();

        let mut draw_2d_graph = RenderGraph::default();
        draw_2d_graph.add_node(graph::node::MAIN_PASS, pass_node_2d);
        let input_node_id = draw_2d_graph.set_input(vec![SlotInfo::new(
            graph::input::VIEW_ENTITY,
            SlotType::Entity,
        )]);
        draw_2d_graph
            .add_slot_edge(
                input_node_id,
                graph::input::VIEW_ENTITY,
                graph::node::MAIN_PASS,
                MainPass2dNode::IN_VIEW,
            )
            .unwrap();
        graph.add_sub_graph(graph::NAME, draw_2d_graph);
    }
}

pub struct Transparent2d {
    pub sort_key: FloatOrd,
    pub entity: Entity,
    pub pipeline: CachedRenderPipelineId,
    pub draw_function: DrawFunctionId,
    /// Range in the vertex buffer of this item
    pub batch_range: Option<Range<u32>>,
}

impl PhaseItem for Transparent2d {
    type SortKey = FloatOrd;

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
        items.sort_by_key(|item| item.sort_key());
    }
}

impl EntityPhaseItem for Transparent2d {
    #[inline]
    fn entity(&self) -> Entity {
        self.entity
    }
}

impl CachedRenderPipelinePhaseItem for Transparent2d {
    #[inline]
    fn cached_pipeline(&self) -> CachedRenderPipelineId {
        self.pipeline
    }
}

impl BatchedPhaseItem for Transparent2d {
    fn batch_range(&self) -> &Option<Range<u32>> {
        &self.batch_range
    }

    fn batch_range_mut(&mut self) -> &mut Option<Range<u32>> {
        &mut self.batch_range
    }
}

pub fn extract_core_2d_camera_phases(
    mut commands: Commands,
    cameras_2d: Extract<Query<(Entity, &Camera), With<Camera2d>>>,
) {
    for (entity, camera) in cameras_2d.iter() {
        if camera.is_active {
            commands
                .get_or_spawn(entity)
                .insert(RenderPhase::<Transparent2d>::default());
        }
    }
}
