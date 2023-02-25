use std::cmp::Reverse;

use bevy_app::Plugin;
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
use core_3d::Camera3d;

use crate::core_3d;

use self::node::Gizmo3dNode;

mod node;

pub struct Gizmo3dPlugin;

impl Plugin for Gizmo3dPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else { return; };

        render_app
            .init_resource::<DrawFunctions<GizmoLine3d>>()
            .add_system(sort_phase_system::<GizmoLine3d>)
            .add_system_to_schedule(ExtractSchedule, extract_gizmo_line_3d_camera_phase);

        let gizmo_node = Gizmo3dNode::new(&mut render_app.world);
        let mut binding = render_app.world.resource_mut::<RenderGraph>();
        let graph = binding.get_sub_graph_mut(core_3d::graph::NAME).unwrap();

        graph.add_node(core_3d::graph::node::GIZMO, gizmo_node);
        graph.add_slot_edge(
            graph.input_node().id,
            core_3d::graph::input::VIEW_ENTITY,
            core_3d::graph::node::GIZMO,
            Gizmo3dNode::IN_VIEW,
        );
        graph.add_node_edge(
            core_3d::graph::node::END_MAIN_PASS_POST_PROCESSING,
            core_3d::graph::node::GIZMO,
        );
        graph.add_node_edge(core_3d::graph::node::GIZMO, core_3d::graph::node::UPSCALING);
    }
}

pub struct GizmoLine3d {
    pub distance: f32,
    pub pipeline: CachedRenderPipelineId,
    pub entity: Entity,
    pub draw_function: DrawFunctionId,
}

impl PhaseItem for GizmoLine3d {
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

impl CachedRenderPipelinePhaseItem for GizmoLine3d {
    #[inline]
    fn cached_pipeline(&self) -> CachedRenderPipelineId {
        self.pipeline
    }
}

pub fn extract_gizmo_line_3d_camera_phase(
    mut commands: Commands,
    cameras_3d: Extract<Query<(Entity, &Camera), With<Camera3d>>>,
) {
    for (entity, camera) in &cameras_3d {
        if camera.is_active {
            commands
                .get_or_spawn(entity)
                .insert(RenderPhase::<GizmoLine3d>::default());
        }
    }
}
