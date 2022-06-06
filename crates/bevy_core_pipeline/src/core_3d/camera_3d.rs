use crate::clear_color::ClearColorConfig;
use bevy_ecs::{prelude::*, query::QueryItem};
use bevy_reflect::Reflect;
use bevy_render::{
    camera::{Camera, CameraRenderGraph, Projection},
    extract_component::ExtractComponent,
    primitives::Frustum,
    view::VisibleEntities,
};
use bevy_transform::prelude::{GlobalTransform, Transform};

#[derive(Component, Default, Reflect, Clone)]
#[reflect(Component)]
pub struct Camera3d {
    pub clear_color: ClearColorConfig,
}

impl ExtractComponent for Camera3d {
    type Query = &'static Self;
    type Filter = With<Camera>;

    fn extract_component(item: QueryItem<Self::Query>) -> Self {
        item.clone()
    }
}

#[derive(Bundle)]
pub struct Camera3dBundle {
    pub camera: Camera,
    pub camera_render_graph: CameraRenderGraph,
    pub projection: Projection,
    pub visible_entities: VisibleEntities,
    pub frustum: Frustum,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub camera_3d: Camera3d,
}

// NOTE: ideally Perspective and Orthographic defaults can share the same impl, but sadly it breaks rust's type inference
impl Default for Camera3dBundle {
    fn default() -> Self {
        Self {
            camera_render_graph: CameraRenderGraph::new(crate::core_3d::graph::NAME),
            camera: Default::default(),
            projection: Default::default(),
            visible_entities: Default::default(),
            frustum: Default::default(),
            transform: Default::default(),
            global_transform: Default::default(),
            camera_3d: Default::default(),
        }
    }
}
