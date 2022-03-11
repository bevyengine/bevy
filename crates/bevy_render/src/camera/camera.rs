use crate::{
    camera::CameraProjection,
    prelude::Image,
    primitives::{Line, Plane},
    render_asset::RenderAssets,
    render_resource::TextureView,
    view::ExtractedWindows,
};
use bevy_asset::{AssetEvent, Assets, Handle};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    event::EventReader,
    prelude::{DetectChanges, QueryState},
    query::Added,
    reflect::ReflectComponent,
    system::{QuerySet, Res},
};
use bevy_math::{Mat4, UVec2, Vec2, Vec3, Vec4};
use bevy_reflect::{Reflect, ReflectDeserialize};
use bevy_transform::components::GlobalTransform;
use bevy_utils::HashSet;
use bevy_window::{WindowCreated, WindowId, WindowResized, Windows};
use serde::{Deserialize, Serialize};
use wgpu::Extent3d;

#[derive(Component, Default, Debug, Reflect)]
#[reflect(Component)]
pub struct Camera {
    pub projection_matrix: Mat4,
    pub name: Option<String>,
    #[reflect(ignore)]
    pub target: RenderTarget,
    #[reflect(ignore)]
    pub depth_calculation: DepthCalculation,
    pub near: f32,
    pub far: f32,
}

#[derive(Debug, Clone, Reflect, PartialEq, Eq, Hash)]
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

impl Camera {
    /// Given a position in world space, use the camera to compute the screen space coordinates.
    pub fn world_to_screen(
        &self,
        windows: &Windows,
        images: &Assets<Image>,
        camera_transform: &GlobalTransform,
        world_position: Vec3,
    ) -> Option<Vec2> {
        let window_size = self.target.get_logical_size(windows, images)?;
        // Build a transform to convert from world to NDC using camera data
        let world_to_ndc: Mat4 =
            self.projection_matrix * camera_transform.compute_matrix().inverse();
        let ndc_space_coords: Vec3 = world_to_ndc.project_point3(world_position);
        // NDC z-values outside of 0 < z < 1 are outside the camera frustum and are thus not in screen space
        if ndc_space_coords.z < 0.0 || ndc_space_coords.z > 1.0 {
            return None;
        }
        // Once in NDC space, we can discard the z element and rescale x/y to fit the screen
        let screen_space_coords = (ndc_space_coords.truncate() + Vec2::ONE) / 2.0 * window_size;
        if !screen_space_coords.is_nan() {
            Some(screen_space_coords)
        } else {
            None
        }
    }

    /// Given a position in screen space, compute the world-space line that corresponds to it.
    pub fn screen_to_world_ray(
        &self,
        pos_screen: Vec2,
        windows: &Windows,
        images: &Assets<Image>,
        camera_transform: &GlobalTransform,
    ) -> Line {
        let view_matrix = camera_transform.compute_matrix();
        let window_size = self.target.get_logical_size(windows, images).unwrap();
        let projection_matrix = self.projection_matrix;

        // Normalized device coordinate cursor position from (-1, -1, 1) to (1, 1, 0) where 0 is at the far plane
        // and 1 is at the near plane.
        let cursor_ndc = (pos_screen / window_size) * 2.0 - Vec2::ONE;
        let cursor_pos_ndc_near: Vec3 = cursor_ndc.extend(1.0);
        let cursor_pos_ndc_far: Vec3 = cursor_ndc.extend(0.0);

        // Use near and far ndc points to generate a ray in world space
        // This method is more robust than using the location of the camera as the start of
        // the ray, because ortho cameras have a focal point at infinity!
        let inverse_projection = projection_matrix.inverse();
        let cursor_pos_view_near = inverse_projection.project_point3(cursor_pos_ndc_near);
        let cursor_pos_view_far = inverse_projection.project_point3(cursor_pos_ndc_far);
        let cursor_pos_near = view_matrix.transform_point3(cursor_pos_view_near);
        let cursor_pos_far = view_matrix.transform_point3(cursor_pos_view_far);
        let ray_direction = (cursor_pos_far - cursor_pos_near).normalize();
        Line::from_point_direction(cursor_pos_near, ray_direction)
    }

    /// Given a position in screen space and a plane in world space, compute what point on the plane the point in screen space corresponds to.
    /// In 2D, use `screen_to_point_2d`.
    pub fn screen_to_point_on_plane(
        &self,
        pos_screen: Vec2,
        plane: Plane,
        windows: &Windows,
        images: &Assets<Image>,
        camera_transform: &GlobalTransform,
    ) -> Option<Vec3> {
        let world_ray = self.screen_to_world_ray(pos_screen, windows, images, camera_transform);
        let plane_normal = plane.normal();
        let direction_dot_normal = world_ray.direction.dot(plane_normal);
        if world_ray.point.extend(1.0).dot(plane.normal_d()).abs() < f32::EPSILON {
            Some(world_ray.point)
        } else if direction_dot_normal.abs() < f32::EPSILON {
            None
        } else {
            // https://www.scratchapixel.com/lessons/3d-basic-rendering/minimal-ray-tracer-rendering-simple-shapes/ray-plane-and-ray-disk-intersection
            let p0 = plane_normal * plane.d();
            let t = (p0 - world_ray.point).dot(plane_normal) / direction_dot_normal;
            Some(world_ray.point + t * world_ray.direction)
        }
    }

    /// Computes the world position for a given screen position.
    /// The output will always be on the XY plane with Z at zero. It is designed for 2D, but also works with a 3D camera.
    /// For more flexibility in 3D, consider `screen_to_point_on_plane`.
    pub fn screen_to_point_2d(
        &self,
        pos_screen: Vec2,
        windows: &Windows,
        images: &Assets<Image>,
        camera_transform: &GlobalTransform,
    ) -> Option<Vec3> {
        self.screen_to_point_on_plane(
            pos_screen,
            Plane::new(Vec4::new(0., 0., 1., 0.)),
            windows,
            images,
            camera_transform,
        )
    }
}

#[allow(clippy::type_complexity)]
pub fn camera_system<T: CameraProjection + Component>(
    mut window_resized_events: EventReader<WindowResized>,
    mut window_created_events: EventReader<WindowCreated>,
    mut image_asset_events: EventReader<AssetEvent<Image>>,
    windows: Res<Windows>,
    images: Res<Assets<Image>>,
    mut queries: QuerySet<(
        QueryState<(Entity, &mut Camera, &mut T)>,
        QueryState<Entity, Added<Camera>>,
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
    for entity in &mut queries.q1().iter() {
        added_cameras.push(entity);
    }
    for (entity, mut camera, mut camera_projection) in queries.q0().iter_mut() {
        if camera
            .target
            .is_changed(&changed_window_ids, &changed_image_handles)
            || added_cameras.contains(&entity)
            || camera_projection.is_changed()
        {
            if let Some(size) = camera.target.get_logical_size(&windows, &images) {
                camera_projection.update(size.x, size.y);
                camera.projection_matrix = camera_projection.get_projection_matrix();
                camera.depth_calculation = camera_projection.depth_calculation();
            }
        }
    }
}
