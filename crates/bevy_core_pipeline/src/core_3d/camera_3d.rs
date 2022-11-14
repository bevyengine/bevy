use crate::{clear_color::ClearColorConfig, tonemapping::Tonemapping};
use bevy_ecs::{prelude::*, query::QueryItem};
use bevy_reflect::{Reflect, ReflectDeserialize, ReflectSerialize};
use bevy_render::{
    camera::{Camera, CameraRenderGraph, Projection},
    extract_component::ExtractComponent,
    primitives::Frustum,
    render_resource::LoadOp,
    view::VisibleEntities,
};
use bevy_transform::prelude::{GlobalTransform, Transform};
use serde::{Deserialize, Serialize};

/// Configuration for the "main 3d render graph".
#[derive(Component, Reflect, Clone, Default)]
#[reflect(Component)]
pub struct Camera3d {
    /// The clear color operation to perform for the main 3d pass.
    pub clear_color: ClearColorConfig,
    /// The depth clear operation to perform for the main 3d pass.
    pub depth_load_op: Camera3dDepthLoadOp,
}

/// The depth clear operation to perform for the main 3d pass.
#[derive(Reflect, Serialize, Deserialize, Clone, Debug)]
#[reflect(Serialize, Deserialize)]
pub enum Camera3dDepthLoadOp {
    /// Clear with a specified value.
    /// Note that 0.0 is the far plane due to bevy's use of reverse-z projections.
    Clear(f32),
    /// Load from memory.
    Load,
}

impl Default for Camera3dDepthLoadOp {
    fn default() -> Self {
        Camera3dDepthLoadOp::Clear(0.0)
    }
}

impl From<Camera3dDepthLoadOp> for LoadOp<f32> {
    fn from(config: Camera3dDepthLoadOp) -> Self {
        match config {
            Camera3dDepthLoadOp::Clear(x) => LoadOp::Clear(x),
            Camera3dDepthLoadOp::Load => LoadOp::Load,
        }
    }
}

impl ExtractComponent for Camera3d {
    type Query = &'static Self;
    type Filter = With<Camera>;

    fn extract_component(item: QueryItem<'_, Self::Query>) -> Self {
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
    pub tonemapping: Tonemapping,
}

// NOTE: ideally Perspective and Orthographic defaults can share the same impl, but sadly it breaks rust's type inference
impl Default for Camera3dBundle {
    fn default() -> Self {
        Self {
            camera_render_graph: CameraRenderGraph::new(crate::core_3d::graph::NAME),
            tonemapping: Tonemapping::Enabled {
                deband_dither: true,
            },
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
