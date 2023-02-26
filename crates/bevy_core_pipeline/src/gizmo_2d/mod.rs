use bevy_app::{IntoSystemAppConfig, Plugin};
use bevy_ecs::{
    prelude::Entity,
    query::With,
    system::{Commands, Query},
};
use bevy_render::{
    prelude::Camera,
    render_graph::RenderGraph,
    render_phase::{
        sort_phase_system, CachedRenderPipelinePhaseItem, DrawFunctionId, DrawFunctions, PhaseItem,
        RenderPhase,
    },
    render_resource::*,
    Extract, ExtractSchedule, RenderApp,
};
use bevy_utils::FloatOrd;

use crate::core_2d::{self, Camera2d};

use self::node::Gizmo2dNode;

pub mod node;

pub struct Gizmo2dPlugin;

impl Plugin for Gizmo2dPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else { return; };

        render_app
            .init_resource::<DrawFunctions<GizmoLine2d>>()
            .add_system(sort_phase_system::<GizmoLine2d>)
            .add_system(extract_gizmo_line_2d_camera_phase.in_schedule(ExtractSchedule));

        let gizmo_node = Gizmo2dNode::new(&mut render_app.world);
        let mut binding = render_app.world.resource_mut::<RenderGraph>();
        let graph = binding.get_sub_graph_mut(core_2d::graph::NAME).unwrap();

        graph.add_node(core_2d::graph::node::GIZMO, gizmo_node);
        graph.add_slot_edge(
            graph.input_node().id,
            core_2d::graph::input::VIEW_ENTITY,
            core_2d::graph::node::GIZMO,
            Gizmo2dNode::IN_VIEW,
        );
        graph.add_node_edge(
            core_2d::graph::node::END_MAIN_PASS_POST_PROCESSING,
            core_2d::graph::node::GIZMO,
        );
        graph.add_node_edge(core_2d::graph::node::GIZMO, core_2d::graph::node::UPSCALING);
    }
}

pub struct GizmoLine2d {
    pub sort_key: FloatOrd,
    pub pipeline: CachedRenderPipelineId,
    pub entity: Entity,
    pub draw_function: DrawFunctionId,
}

impl PhaseItem for GizmoLine2d {
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
        items.sort_by_key(|item| item.sort_key());
    }
}

impl CachedRenderPipelinePhaseItem for GizmoLine2d {
    #[inline]
    fn cached_pipeline(&self) -> CachedRenderPipelineId {
        self.pipeline
    }
}

pub fn extract_gizmo_line_2d_camera_phase(
    mut commands: Commands,
    cameras_2d: Extract<Query<(Entity, &Camera), With<Camera2d>>>,
) {
    for (entity, camera) in &cameras_2d {
        if camera.is_active {
            commands
                .get_or_spawn(entity)
                .insert(RenderPhase::<GizmoLine2d>::default());
        }
    }
}
