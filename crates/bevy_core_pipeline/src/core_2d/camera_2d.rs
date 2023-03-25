#![expect(deprecated)]

use crate::{
    core_2d::graph::Core2d,
    tonemapping::{DebandDither, Tonemapping},
};
use bevy_ecs::prelude::*;
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_render::sync_world::SyncToRenderWorld;
use bevy_render::{
    camera::{
        Camera, CameraMainTextureUsages, CameraProjection, CameraRenderGraph,
        OrthographicProjection,
    },
    extract_component::ExtractComponent,
    prelude::Msaa,
    primitives::Frustum,
    view::VisibleEntities,
};
use bevy_transform::prelude::{GlobalTransform, Transform};

/// A 2D camera component. Enables the 2D render graph for a [`Camera`].
#[derive(Component, Default, Reflect, Clone, ExtractComponent)]
#[extract_component_filter(With<Camera>)]
#[reflect(Component, Default)]
#[require(
    Camera,
    DebandDither,
    CameraRenderGraph(|| CameraRenderGraph::new(Core2d)),
    OrthographicProjection(OrthographicProjection::default_2d),
    Frustum(|| OrthographicProjection::default_2d().compute_frustum(&GlobalTransform::from(Transform::default()))),
    Tonemapping(|| Tonemapping::None),
)]
pub struct Camera2d;

#[derive(Bundle, Clone)]
#[deprecated(
    since = "0.15.0",
    note = "Use the `Camera2d` component instead. Inserting it will now also insert the other components required by it automatically."
)]
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
    pub main_texture_usages: CameraMainTextureUsages,
    pub msaa: Msaa,
    /// Marker component that indicates that its entity needs to be synchronized to the render world
    pub sync: SyncToRenderWorld,
}

impl Default for Camera2dBundle {
    fn default() -> Self {
        let projection = OrthographicProjection::default_2d();
        let transform = Transform::default();
        let frustum = projection.compute_frustum(&GlobalTransform::from(transform));
        Self {
            camera_render_graph: CameraRenderGraph::new(Core2d),
            projection,
            visible_entities: VisibleEntities::default(),
            frustum,
            transform,
            global_transform: Default::default(),
            camera: Camera::default(),
            camera_2d: Camera2d,
            tonemapping: Tonemapping::None,
            deband_dither: DebandDither::Disabled,
            main_texture_usages: Default::default(),
            msaa: Default::default(),
            sync: Default::default(),
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
            far: Some(far),
            ..OrthographicProjection::default_2d()
        };
        let transform = Transform::from_xyz(0.0, 0.0, far - 0.1);
        let frustum = projection.compute_frustum(&GlobalTransform::from(transform));
        Self {
            camera_render_graph: CameraRenderGraph::new(Core2d),
            projection,
            visible_entities: VisibleEntities::default(),
            frustum,
            transform,
            global_transform: Default::default(),
            camera: Camera::default(),
            camera_2d: Camera2d,
            tonemapping: Tonemapping::None,
            deband_dither: DebandDither::Disabled,
            main_texture_usages: Default::default(),
            msaa: Default::default(),
            sync: Default::default(),
        }
    }
}
