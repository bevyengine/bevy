use crate::{
    camera::CameraProjection,
    prelude::Image,
    render_asset::RenderAssets,
    render_resource::TextureView,
    view::{ExtractedView, ExtractedWindows, VisibleEntities},
};
use bevy_asset::{AssetEvent, Assets, Handle};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    change_detection::DetectChanges,
    component::Component,
    entity::Entity,
    event::EventReader,
    query::Added,
    reflect::ReflectComponent,
    system::{Commands, ParamSet, Query, Res},
};
use bevy_math::{Mat4, UVec2, Vec2, Vec3};
use bevy_reflect::prelude::*;
use bevy_transform::components::GlobalTransform;
use bevy_utils::HashSet;
use bevy_window::{WindowCreated, WindowId, WindowResized, Windows};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use wgpu::Extent3d;

#[derive(Component, Debug, Reflect, Clone)]
#[reflect(Component)]
pub struct Camera {
    pub projection_matrix: Mat4,
    pub logical_target_size: Option<Vec2>,
    pub physical_target_size: Option<UVec2>,
    pub priority: isize,
    pub is_active: bool,
    #[reflect(ignore)]
    pub target: RenderTarget,
    #[reflect(ignore)]
    pub depth_calculation: DepthCalculation,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            is_active: true,
            priority: 0,
            projection_matrix: Default::default(),
            logical_target_size: Default::default(),
            physical_target_size: Default::default(),
            target: Default::default(),
            depth_calculation: Default::default(),
        }
    }
}

impl Camera {
    /// Given a position in world space, use the camera to compute the viewport-space coordinates.
    ///
    /// To get the coordinates in Normalized Device Coordinates, you should use
    /// [`world_to_ndc`](Self::world_to_ndc).
    pub fn world_to_viewport(
        &self,
        camera_transform: &GlobalTransform,
        world_position: Vec3,
    ) -> Option<Vec2> {
        let target_size = self.logical_target_size?;
        let ndc_space_coords = self.world_to_ndc(camera_transform, world_position)?;
        // NDC z-values outside of 0 < z < 1 are outside the camera frustum and are thus not in viewport-space
        if ndc_space_coords.z < 0.0 || ndc_space_coords.z > 1.0 {
            return None;
        }

        // Once in NDC space, we can discard the z element and rescale x/y to fit the screen
        Some((ndc_space_coords.truncate() + Vec2::ONE) / 2.0 * target_size)
    }

    /// Given a position in world space, use the camera's viewport to compute the Normalized Device Coordinates.
    ///
    /// Values returned will be between -1.0 and 1.0 when the position is within the viewport.
    /// To get the coordinates in the render target's viewport dimensions, you should use
    /// [`world_to_viewport`](Self::world_to_viewport).
    pub fn world_to_ndc(
        &self,
        camera_transform: &GlobalTransform,
        world_position: Vec3,
    ) -> Option<Vec3> {
        // Build a transform to convert from world to NDC using camera data
        let world_to_ndc: Mat4 =
            self.projection_matrix * camera_transform.compute_matrix().inverse();
        let ndc_space_coords: Vec3 = world_to_ndc.project_point3(world_position);

        if !ndc_space_coords.is_nan() {
            Some(ndc_space_coords)
        } else {
            None
        }
    }
}

/// Configures the [`RenderGraph`](crate::render_graph::RenderGraph) name assigned to be run for a given [`Camera`] entity.
#[derive(Component, Deref, DerefMut, Reflect, Default)]
#[reflect(Component)]
pub struct CameraRenderGraph(Cow<'static, str>);

impl CameraRenderGraph {
    #[inline]
    pub fn new<T: Into<Cow<'static, str>>>(name: T) -> Self {
        Self(name.into())
    }
}

#[derive(Debug, Clone, Reflect, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum RenderTarget {
    /// Window to which the camera's view is rendered.
    Window(WindowId),
    /// Image to which the camera's view is rendered.
    Image(Handle<Image>),
}

impl Default for RenderTarget {
    fn default() -> Self {
        Self::Window(Default::default())
    }
}

impl RenderTarget {
    pub fn get_texture_view<'a>(
        &self,
        windows: &'a ExtractedWindows,
        images: &'a RenderAssets<Image>,
    ) -> Option<&'a TextureView> {
        match self {
            RenderTarget::Window(window_id) => windows
                .get(window_id)
                .and_then(|window| window.swap_chain_texture.as_ref()),
            RenderTarget::Image(image_handle) => {
                images.get(image_handle).map(|image| &image.texture_view)
            }
        }
    }
    pub fn get_physical_size(&self, windows: &Windows, images: &Assets<Image>) -> Option<UVec2> {
        match self {
            RenderTarget::Window(window_id) => windows
                .get(*window_id)
                .map(|window| UVec2::new(window.physical_width(), window.physical_height())),
            RenderTarget::Image(image_handle) => images.get(image_handle).map(|image| {
                let Extent3d { width, height, .. } = image.texture_descriptor.size;
                UVec2::new(width, height)
            }),
        }
        .filter(|size| size.x > 0 && size.y > 0)
    }
    pub fn get_logical_size(&self, windows: &Windows, images: &Assets<Image>) -> Option<Vec2> {
        match self {
            RenderTarget::Window(window_id) => windows
                .get(*window_id)
                .map(|window| Vec2::new(window.width(), window.height())),
            RenderTarget::Image(image_handle) => images.get(image_handle).map(|image| {
                let Extent3d { width, height, .. } = image.texture_descriptor.size;
                Vec2::new(width as f32, height as f32)
            }),
        }
    }
    // Check if this render target is contained in the given changed windows or images.
    fn is_changed(
        &self,
        changed_window_ids: &[WindowId],
        changed_image_handles: &HashSet<&Handle<Image>>,
    ) -> bool {
        match self {
            RenderTarget::Window(window_id) => changed_window_ids.contains(window_id),
            RenderTarget::Image(image_handle) => changed_image_handles.contains(&image_handle),
        }
    }
}

#[derive(Debug, Clone, Copy, Reflect, Serialize, Deserialize)]
#[reflect_value(Serialize, Deserialize)]
pub enum DepthCalculation {
    /// Pythagorean distance; works everywhere, more expensive to compute.
    Distance,
    /// Optimization for 2D; assuming the camera points towards -Z.
    ZDifference,
}

impl Default for DepthCalculation {
    fn default() -> Self {
        DepthCalculation::Distance
    }
}

pub fn camera_system<T: CameraProjection + Component>(
    mut window_resized_events: EventReader<WindowResized>,
    mut window_created_events: EventReader<WindowCreated>,
    mut image_asset_events: EventReader<AssetEvent<Image>>,
    windows: Res<Windows>,
    images: Res<Assets<Image>>,
    mut queries: ParamSet<(
        Query<(Entity, &mut Camera, &mut T)>,
        Query<Entity, Added<Camera>>,
    )>,
) {
    let mut changed_window_ids = Vec::new();
    // handle resize events. latest events are handled first because we only want to resize each
    // window once
    for event in window_resized_events.iter().rev() {
        if changed_window_ids.contains(&event.id) {
            continue;
        }

        changed_window_ids.push(event.id);
    }

    // handle resize events. latest events are handled first because we only want to resize each
    // window once
    for event in window_created_events.iter().rev() {
        if changed_window_ids.contains(&event.id) {
            continue;
        }

        changed_window_ids.push(event.id);
    }

    let changed_image_handles: HashSet<&Handle<Image>> = image_asset_events
        .iter()
        .filter_map(|event| {
            if let AssetEvent::Modified { handle } = event {
                Some(handle)
            } else {
                None
            }
        })
        .collect();

    let mut added_cameras = vec![];
    for entity in &mut queries.p1().iter() {
        added_cameras.push(entity);
    }
    for (entity, mut camera, mut camera_projection) in queries.p0().iter_mut() {
        if camera
            .target
            .is_changed(&changed_window_ids, &changed_image_handles)
            || added_cameras.contains(&entity)
            || camera_projection.is_changed()
        {
            camera.logical_target_size = camera.target.get_logical_size(&windows, &images);
            camera.physical_target_size = camera.target.get_physical_size(&windows, &images);
            if let Some(size) = camera.logical_target_size {
                camera_projection.update(size.x, size.y);
                camera.projection_matrix = camera_projection.get_projection_matrix();
                camera.depth_calculation = camera_projection.depth_calculation();
            }
        }
    }
}

#[derive(Component, Debug)]
pub struct ExtractedCamera {
    pub target: RenderTarget,
    pub physical_size: Option<UVec2>,
    pub render_graph: Cow<'static, str>,
    pub priority: isize,
}

pub fn extract_cameras(
    mut commands: Commands,
    query: Query<(
        Entity,
        &Camera,
        &CameraRenderGraph,
        &GlobalTransform,
        &VisibleEntities,
    )>,
) {
    for (entity, camera, camera_render_graph, transform, visible_entities) in query.iter() {
        if !camera.is_active {
            continue;
        }
        if let Some(size) = camera.physical_target_size {
            commands.get_or_spawn(entity).insert_bundle((
                ExtractedCamera {
                    target: camera.target.clone(),
                    physical_size: Some(size),
                    render_graph: camera_render_graph.0.clone(),
                    priority: camera.priority,
                },
                ExtractedView {
                    projection: camera.projection_matrix,
                    transform: *transform,
                    width: size.x,
                    height: size.y,
                },
                visible_entities.clone(),
            ));
        }
    }
}
