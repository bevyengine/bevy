use bevy_ecs::{
    entity::Entity,
    prelude::{Bundle, Component, With},
    query::QueryItem,
    system::{Commands, Query},
};
use bevy_render::{
    camera::{Camera, CameraRenderGraph, OrthographicProjection},
    extract_component::ExtractComponent,
    view::VisibleEntities,
};
use bevy_transform::prelude::GlobalTransform;

use super::overlay_node::graph;

pub(crate) fn extract_overlay_camera_phases(
    mut commands: Commands,
    cameras_overlay: Query<(Entity, &Camera), With<CameraOverlay>>,
) {
    for (entity, camera) in cameras_overlay.iter() {
        if camera.is_active {
            commands.get_or_spawn(entity);
        }
    }
}

#[derive(Component, Default, Clone)]
pub struct CameraOverlay {}

impl ExtractComponent for CameraOverlay {
    type Query = &'static Self;
    type Filter = With<Camera>;

    fn extract_component(item: QueryItem<Self::Query>) -> Self {
        item.clone()
    }
}

#[derive(Bundle)]
pub struct CameraOverlayBundle {
    pub camera: Camera,
    pub camera_render_graph: CameraRenderGraph,
    pub projection: OrthographicProjection,
    pub visible_entities: VisibleEntities,
    pub global_transform: GlobalTransform,
    pub camera_overlay: CameraOverlay,
}

impl Default for CameraOverlayBundle {
    fn default() -> Self {
        Self {
            camera: Camera {
                priority: isize::MAX,
                ..Default::default()
            },
            camera_render_graph: CameraRenderGraph::new(graph::NAME),
            visible_entities: Default::default(),
            projection: Default::default(),
            global_transform: Default::default(),
            camera_overlay: Default::default(),
        }
    }
}
