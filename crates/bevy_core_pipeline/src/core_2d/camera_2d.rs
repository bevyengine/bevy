use crate::{
    clear_color::ClearColorConfig,
    tonemapping::{DebandDither, Tonemapping},
};
use bevy_ecs::prelude::*;
use bevy_reflect::Reflect;
use bevy_render::{
    camera::{Camera, CameraProjection, CameraRenderGraph, OrthographicProjection},
    extract_component::ExtractComponent,
    primitives::Frustum,
    view::VisibleEntities,
};
use bevy_transform::prelude::{GlobalTransform, Transform};

#[derive(Component, Default, Reflect, Clone, ExtractComponent)]
#[extract_component_filter(With<Camera>)]
#[reflect(Component)]
pub struct Camera2d {
    pub clear_color: ClearColorConfig,
}

#[derive(Bundle)]
pub struct Camera2dBundle {
    pub camera: Camera,
    pub camera_render_graph: CameraRenderGraph,
    pub projection: OrthographicProjection,
    pub visible_entities: VisibleEntities,
    pub frustum: Frustum,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub camera_2d: Camera2d,
    pub tonemapping: Tonemapping,
    pub deband_dither: DebandDither,
}

impl Default for Camera2dBundle {
    fn default() -> Self {
        let projection = OrthographicProjection {
            far: 1000.,
            near: -1000.,
            ..Default::default()
        };
        let transform = Transform::default();
        let view_projection =
            projection.get_projection_matrix() * transform.compute_matrix().inverse();
        let frustum = Frustum::from_view_projection_custom_far(
            &view_projection,
            &transform.translation,
            &transform.back(),
            projection.far(),
        );
        Self {
            camera_render_graph: CameraRenderGraph::new(crate::core_2d::graph::NAME),
            projection,
            visible_entities: VisibleEntities::default(),
            frustum,
            transform,
            global_transform: Default::default(),
            camera: Camera::default(),
            camera_2d: Camera2d::default(),
            tonemapping: Tonemapping::None,
            deband_dither: DebandDither::Disabled,
        }
    }
}

impl Camera2dBundle {
    /// Create an orthographic projection camera with a custom `Z` position.
    ///
    /// The camera is placed at `Z=far-0.1`, looking toward the world origin `(0,0,0)`.
    /// Its orthographic projection extends from `0.0` to `-far` in camera view space,
    /// corresponding to `Z=far-0.1` (closest to camera) to `Z=-0.1` (furthest away from
    /// camera) in world space.
    pub fn new_with_far(far: f32) -> Self {
        // we want 0 to be "closest" and +far to be "farthest" in 2d, so we offset
        // the camera's translation by far and use a right handed coordinate system
        let projection = OrthographicProjection {
            far,
            ..Default::default()
        };
        let transform = Transform::from_xyz(0.0, 0.0, far - 0.1);
        let view_projection =
            projection.get_projection_matrix() * transform.compute_matrix().inverse();
        let frustum = Frustum::from_view_projection_custom_far(
            &view_projection,
            &transform.translation,
            &transform.back(),
            projection.far(),
        );
        Self {
            camera_render_graph: CameraRenderGraph::new(crate::core_2d::graph::NAME),
            projection,
            visible_entities: VisibleEntities::default(),
            frustum,
            transform,
            global_transform: Default::default(),
            camera: Camera::default(),
            camera_2d: Camera2d::default(),
            tonemapping: Tonemapping::None,
            deband_dither: DebandDither::Disabled,
        }
    }
}
