#![expect(deprecated)]

use crate::core_2d::graph::Core2d;
use crate::tonemapping::{DebandDither, Tonemapping};
use bevy_ecs::prelude::*;
use bevy_reflect::std_traits::ReflectDefault;
use bevy_reflect::Reflect;
use bevy_render::prelude::Msaa;
use bevy_render::{
    camera::{
        Camera, CameraMainTextureUsages, CameraProjection, CameraRenderGraph,
        OrthographicProjection,
    },
    extract_component::ExtractComponent,
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
    CameraRenderGraph(Camera2d::default_render_graph),
    OrthographicProjection(Camera2d::default_projection),
    Frustum(Camera2d::default_frustum),
    Tonemapping(Camera2d::default_tonemapping)
)]
pub struct Camera2d;

impl Camera2d {
    fn default_render_graph() -> CameraRenderGraph {
        CameraRenderGraph::new(Core2d)
    }

    fn default_projection() -> OrthographicProjection {
        OrthographicProjection::default_2d()
    }

    fn default_frustum() -> Frustum {
        Self::default_projection().compute_frustum(&GlobalTransform::from(Transform::default()))
    }

    fn default_tonemapping() -> Tonemapping {
        Tonemapping::None
    }
}

#[derive(Bundle, Clone)]
#[deprecated(
    since = "0.15.0",
    note = "Use `Camera2d` directly instead. Inserting it will now automatically add the other components required by it."
)]
pub struct Camera2dBundle {
    pub camera: Camera,
    pub camera_render_graph: CameraRenderGraph,
    /// Note: default value for `OrthographicProjection.near` is `0.0`
    /// which makes objects on the screen plane invisible to 2D camera.
    /// `Camera2d` sets `near` to negative value,
    /// so be careful when initializing this field manually.
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
        }
    }
}
