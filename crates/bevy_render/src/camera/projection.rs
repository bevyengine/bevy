use core::{
    marker::PhantomData,
    ops::{Div, DivAssign, Mul, MulAssign},
};

use crate::{primitives::Frustum, view::VisibilitySystems};
use bevy_app::{App, Plugin, PostStartup, PostUpdate};
use bevy_ecs::prelude::*;
use bevy_math::{ops, AspectRatio, Mat4, Rect, Vec2, Vec3A, Vec4};
use bevy_reflect::{
    std_traits::ReflectDefault, GetTypeRegistration, Reflect, ReflectDeserialize, ReflectSerialize,
};
use bevy_transform::{components::GlobalTransform, TransformSystem};
use serde::{Deserialize, Serialize};

/// Adds [`Camera`](crate::camera::Camera) driver systems for a given projection type.
///
/// If you are using `bevy_pbr`, then you need to add `PbrProjectionPlugin` along with this.
pub struct CameraProjectionPlugin<T: CameraProjection + Component + GetTypeRegistration>(
    PhantomData<T>,
);
impl<T: CameraProjection + Component + GetTypeRegistration> Plugin for CameraProjectionPlugin<T> {
    fn build(&self, app: &mut App) {
        app.register_type::<T>()
            .add_systems(
                PostStartup,
                crate::camera::camera_system::<T>
                    .in_set(CameraUpdateSystem)
                    // We assume that each camera will only have one projection,
                    // so we can ignore ambiguities with all other monomorphizations.
                    // FIXME: Add an archetype invariant for this https://github.com/bevyengine/bevy/issues/1481.
                    .ambiguous_with(CameraUpdateSystem),
            )
            .add_systems(
                PostUpdate,
                (
                    crate::camera::camera_system::<T>
                        .in_set(CameraUpdateSystem)
                        // We assume that each camera will only have one projection,
                        // so we can ignore ambiguities with all other monomorphizations.
                        // FIXME: Add an archetype invariant for this https://github.com/bevyengine/bevy/issues/1481.
                        .ambiguous_with(CameraUpdateSystem),
                    crate::view::update_frusta::<T>
                        .in_set(VisibilitySystems::UpdateFrusta)
                        .after(crate::camera::camera_system::<T>)
                        .after(TransformSystem::TransformPropagate)
                        // We assume that no camera will have more than one projection component,
                        // so these systems will run independently of one another.
                        // FIXME: Add an archetype invariant for this https://github.com/bevyengine/bevy/issues/1481.
                        .ambiguous_with(VisibilitySystems::UpdateFrusta),
                ),
            );
    }
}
impl<T: CameraProjection + Component + GetTypeRegistration> Default for CameraProjectionPlugin<T> {
    fn default() -> Self {
        Self(Default::default())
    }
}

/// Label for [`camera_system<T>`], shared across all `T`.
///
/// [`camera_system<T>`]: crate::camera::camera_system
#[derive(SystemSet, Clone, Eq, PartialEq, Hash, Debug)]
pub struct CameraUpdateSystem;

/// Trait to control the projection matrix of a camera.
///
/// Components implementing this trait are automatically polled for changes, and used
/// to recompute the camera projection matrix of the [`Camera`] component attached to
/// the same entity as the component implementing this trait.
///
/// Use the plugins [`CameraProjectionPlugin`] and `bevy::pbr::PbrProjectionPlugin` to setup the
/// systems for your [`CameraProjection`] implementation.
///
/// [`Camera`]: crate::camera::Camera
pub trait CameraProjection {
    fn get_clip_from_view(&self) -> Mat4;
    fn get_clip_from_view_for_sub(&self, sub_view: &super::SubCameraView) -> Mat4;
    fn update(&mut self, width: f32, height: f32);
    fn far(&self) -> f32;
    fn get_frustum_corners(&self, z_near: f32, z_far: f32) -> [Vec3A; 8];

    /// Compute camera frustum for camera with given projection and transform.
    ///
    /// This code is called by [`update_frusta`](crate::view::visibility::update_frusta) system
    /// for each camera to update its frustum.
    fn compute_frustum(&self, camera_transform: &GlobalTransform) -> Frustum {
        let clip_from_world =
            self.get_clip_from_view() * camera_transform.compute_matrix().inverse();
        Frustum::from_clip_from_world_custom_far(
            &clip_from_world,
            &camera_transform.translation(),
            &camera_transform.back(),
            self.far(),
        )
    }
}

/// A configurable [`CameraProjection`] that can select its projection type at runtime.
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component, Default, Debug)]
pub enum Projection {
    Perspective(PerspectiveProjection),
    Orthographic(OrthographicProjection),
}

impl From<PerspectiveProjection> for Projection {
    fn from(p: PerspectiveProjection) -> Self {
        Self::Perspective(p)
    }
}

impl From<OrthographicProjection> for Projection {
    fn from(p: OrthographicProjection) -> Self {
        Self::Orthographic(p)
    }
}

impl CameraProjection for Projection {
    fn get_clip_from_view(&self) -> Mat4 {
        match self {
            Projection::Perspective(projection) => projection.get_clip_from_view(),
            Projection::Orthographic(projection) => projection.get_clip_from_view(),
        }
    }

    fn get_clip_from_view_for_sub(&self, sub_view: &super::SubCameraView) -> Mat4 {
        match self {
            Projection::Perspective(projection) => projection.get_clip_from_view_for_sub(sub_view),
            Projection::Orthographic(projection) => projection.get_clip_from_view_for_sub(sub_view),
        }
    }

    fn update(&mut self, width: f32, height: f32) {
        match self {
            Projection::Perspective(projection) => projection.update(width, height),
            Projection::Orthographic(projection) => projection.update(width, height),
        }
    }

    fn far(&self) -> f32 {
        match self {
            Projection::Perspective(projection) => projection.far(),
            Projection::Orthographic(projection) => projection.far(),
        }
    }

    fn get_frustum_corners(&self, z_near: f32, z_far: f32) -> [Vec3A; 8] {
        match self {
            Projection::Perspective(projection) => projection.get_frustum_corners(z_near, z_far),
            Projection::Orthographic(projection) => projection.get_frustum_corners(z_near, z_far),
        }
    }
}

impl Default for Projection {
    fn default() -> Self {
        Projection::Perspective(Default::default())
    }
}

/// A 3D camera projection in which distant objects appear smaller than close objects.
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component, Default, Debug)]
pub struct PerspectiveProjection {
    /// The vertical field of view (FOV) in radians.
    ///
    /// Defaults to a value of π/4 radians or 45 degrees.
    pub fov: f32,

    /// The aspect ratio (width divided by height) of the viewing frustum.
    ///
    /// Bevy's [`camera_system`](crate::camera::camera_system) automatically
    /// updates this value when the aspect ratio of the associated window changes.
    ///
    /// Defaults to a value of `1.0`.
    pub aspect_ratio: f32,

    /// The distance from the camera in world units of the viewing frustum's near plane.
    ///
    /// Objects closer to the camera than this value will not be visible.
    ///
    /// Defaults to a value of `0.1`.
    pub near: f32,

    /// The distance from the camera in world units of the viewing frustum's far plane.
    ///
    /// Objects farther from the camera than this value will not be visible.
    ///
    /// Defaults to a value of `1000.0`.
    pub far: f32,
}

impl CameraProjection for PerspectiveProjection {
    fn get_clip_from_view(&self) -> Mat4 {
        Mat4::perspective_infinite_reverse_rh(self.fov, self.aspect_ratio, self.near)
    }

    fn get_clip_from_view_for_sub(&self, sub_view: &super::SubCameraView) -> Mat4 {
        let full_width = sub_view.full_size.x as f32;
        let full_height = sub_view.full_size.y as f32;
        let sub_width = sub_view.size.x as f32;
        let sub_height = sub_view.size.y as f32;
        let offset_x = sub_view.offset.x;
        // Y-axis increases from top to bottom
        let offset_y = full_height - (sub_view.offset.y + sub_height);

        // Original frustum parameters
        let top = self.near * ops::tan(0.5 * self.fov);
        let bottom = -top;
        let right = top * self.aspect_ratio;
        let left = -right;

        // Calculate scaling factors
        let width = right - left;
        let height = top - bottom;

        // Calculate the new frustum parameters
        let left_prime = left + (width * offset_x) / full_width;
        let right_prime = left + (width * (offset_x + sub_width)) / full_width;
        let bottom_prime = bottom + (height * offset_y) / full_height;
        let top_prime = bottom + (height * (offset_y + sub_height)) / full_height;

        // Compute the new projection matrix
        let x = (2.0 * self.near) / (right_prime - left_prime);
        let y = (2.0 * self.near) / (top_prime - bottom_prime);
        let a = (right_prime + left_prime) / (right_prime - left_prime);
        let b = (top_prime + bottom_prime) / (top_prime - bottom_prime);

        Mat4::from_cols(
            Vec4::new(x, 0.0, 0.0, 0.0),
            Vec4::new(0.0, y, 0.0, 0.0),
            Vec4::new(a, b, 0.0, -1.0),
            Vec4::new(0.0, 0.0, self.near, 0.0),
        )
    }

    fn update(&mut self, width: f32, height: f32) {
        self.aspect_ratio = AspectRatio::try_new(width, height)
            .expect("Failed to update PerspectiveProjection: width and height must be positive, non-zero values")
            .ratio();
    }

    fn far(&self) -> f32 {
        self.far
    }

    fn get_frustum_corners(&self, z_near: f32, z_far: f32) -> [Vec3A; 8] {
        let tan_half_fov = ops::tan(self.fov / 2.);
        let a = z_near.abs() * tan_half_fov;
        let b = z_far.abs() * tan_half_fov;
        let aspect_ratio = self.aspect_ratio;
        // NOTE: These vertices are in the specific order required by [`calculate_cascade`].
        [
            Vec3A::new(a * aspect_ratio, -a, z_near),  // bottom right
            Vec3A::new(a * aspect_ratio, a, z_near),   // top right
            Vec3A::new(-a * aspect_ratio, a, z_near),  // top left
            Vec3A::new(-a * aspect_ratio, -a, z_near), // bottom left
            Vec3A::new(b * aspect_ratio, -b, z_far),   // bottom right
            Vec3A::new(b * aspect_ratio, b, z_far),    // top right
            Vec3A::new(-b * aspect_ratio, b, z_far),   // top left
            Vec3A::new(-b * aspect_ratio, -b, z_far),  // bottom left
        ]
    }
}

impl Default for PerspectiveProjection {
    fn default() -> Self {
        PerspectiveProjection {
            fov: core::f32::consts::PI / 4.0,
            near: 0.1,
            far: 1000.0,
            aspect_ratio: 1.0,
        }
    }
}

/// Scaling mode for [`OrthographicProjection`].
///
/// # Examples
///
/// Configure the orthographic projection to two world units per window height:
///
/// ```
/// # use bevy_render::camera::{OrthographicProjection, Projection, ScalingMode};
/// let projection = Projection::Orthographic(OrthographicProjection {
///    scaling_mode: ScalingMode::FixedVertical(2.0),
///    ..OrthographicProjection::default_2d()
/// });
/// ```
#[derive(Debug, Clone, Copy, Reflect, Serialize, Deserialize)]
#[reflect(Serialize, Deserialize)]
pub enum ScalingMode {
    /// Manually specify the projection's size, ignoring window resizing. The image will stretch.
    /// Arguments are in world units.
    Fixed { width: f32, height: f32 },
    /// Match the viewport size.
    /// The argument is the number of pixels that equals one world unit.
    WindowSize(f32),
    /// Keeping the aspect ratio while the axes can't be smaller than given minimum.
    /// Arguments are in world units.
    AutoMin { min_width: f32, min_height: f32 },
    /// Keeping the aspect ratio while the axes can't be bigger than given maximum.
    /// Arguments are in world units.
    AutoMax { max_width: f32, max_height: f32 },
    /// Keep the projection's height constant; width will be adjusted to match aspect ratio.
    /// The argument is the desired height of the projection in world units.
    FixedVertical(f32),
    /// Keep the projection's width constant; height will be adjusted to match aspect ratio.
    /// The argument is the desired width of the projection in world units.
    FixedHorizontal(f32),
}

impl Mul<f32> for ScalingMode {
    type Output = ScalingMode;

    /// Scale the `ScalingMode`. For example, multiplying by 2 makes the viewport twice as large.
    fn mul(self, rhs: f32) -> ScalingMode {
        match self {
            ScalingMode::Fixed { width, height } => ScalingMode::Fixed {
                width: width * rhs,
                height: height * rhs,
            },
            ScalingMode::WindowSize(pixels_per_world_unit) => {
                ScalingMode::WindowSize(pixels_per_world_unit / rhs)
            }
            ScalingMode::AutoMin {
                min_width,
                min_height,
            } => ScalingMode::AutoMin {
                min_width: min_width * rhs,
                min_height: min_height * rhs,
            },
            ScalingMode::AutoMax {
                max_width,
                max_height,
            } => ScalingMode::AutoMax {
                max_width: max_width * rhs,
                max_height: max_height * rhs,
            },
            ScalingMode::FixedVertical(size) => ScalingMode::FixedVertical(size * rhs),
            ScalingMode::FixedHorizontal(size) => ScalingMode::FixedHorizontal(size * rhs),
        }
    }
}

impl MulAssign<f32> for ScalingMode {
    fn mul_assign(&mut self, rhs: f32) {
        *self = *self * rhs;
    }
}

impl Div<f32> for ScalingMode {
    type Output = ScalingMode;

    /// Scale the `ScalingMode`. For example, dividing by 2 makes the viewport half as large.
    fn div(self, rhs: f32) -> ScalingMode {
        self * (1.0 / rhs)
    }
}

impl DivAssign<f32> for ScalingMode {
    fn div_assign(&mut self, rhs: f32) {
        *self = *self / rhs;
    }
}

/// Project a 3D space onto a 2D surface using parallel lines, i.e., unlike [`PerspectiveProjection`],
/// the size of objects remains the same regardless of their distance to the camera.
///
/// The volume contained in the projection is called the *view frustum*. Since the viewport is rectangular
/// and projection lines are parallel, the view frustum takes the shape of a cuboid.
///
/// Note that the scale of the projection and the apparent size of objects are inversely proportional.
/// As the size of the projection increases, the size of objects decreases.
///
/// # Examples
///
/// Configure the orthographic projection to one world unit per 100 window pixels:
///
/// ```
/// # use bevy_render::camera::{OrthographicProjection, Projection, ScalingMode};
/// let projection = Projection::Orthographic(OrthographicProjection {
///     scaling_mode: ScalingMode::WindowSize(100.0),
///     ..OrthographicProjection::default_2d()
/// });
/// ```
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component, Debug, FromWorld)]
pub struct OrthographicProjection {
    /// The distance of the near clipping plane in world units.
    ///
    /// Objects closer than this will not be rendered.
    ///
    /// Defaults to `0.0`
    pub near: f32,
    /// The distance of the far clipping plane in world units.
    ///
    /// Objects further than this will not be rendered.
    ///
    /// Defaults to `1000.0`
    pub far: f32,
    /// Specifies the origin of the viewport as a normalized position from 0 to 1, where (0, 0) is the bottom left
    /// and (1, 1) is the top right. This determines where the camera's position sits inside the viewport.
    ///
    /// When the projection scales due to viewport resizing, the position of the camera, and thereby `viewport_origin`,
    /// remains at the same relative point.
    ///
    /// Consequently, this is pivot point when scaling. With a bottom left pivot, the projection will expand
    /// upwards and to the right. With a top right pivot, the projection will expand downwards and to the left.
    /// Values in between will caused the projection to scale proportionally on each axis.
    ///
    /// Defaults to `(0.5, 0.5)`, which makes scaling affect opposite sides equally, keeping the center
    /// point of the viewport centered.
    pub viewport_origin: Vec2,
    /// How the projection will scale to the viewport.
    ///
    /// Defaults to `ScalingMode::WindowSize(1.0)`
    pub scaling_mode: ScalingMode,
    /// The area that the projection covers relative to `viewport_origin`.
    ///
    /// Bevy's [`camera_system`](crate::camera::camera_system) automatically
    /// updates this value when the viewport is resized depending on `OrthographicProjection`'s other fields.
    /// In this case, `area` should not be manually modified.
    ///
    /// It may be necessary to set this manually for shadow projections and such.
    pub area: Rect,
}

impl CameraProjection for OrthographicProjection {
    fn get_clip_from_view(&self) -> Mat4 {
        Mat4::orthographic_rh(
            self.area.min.x,
            self.area.max.x,
            self.area.min.y,
            self.area.max.y,
            // NOTE: near and far are swapped to invert the depth range from [0,1] to [1,0]
            // This is for interoperability with pipelines using infinite reverse perspective projections.
            self.far,
            self.near,
        )
    }

    fn get_clip_from_view_for_sub(&self, sub_view: &super::SubCameraView) -> Mat4 {
        let full_width = sub_view.full_size.x as f32;
        let full_height = sub_view.full_size.y as f32;
        let offset_x = sub_view.offset.x;
        let offset_y = sub_view.offset.y;
        let sub_width = sub_view.size.x as f32;
        let sub_height = sub_view.size.y as f32;

        // Orthographic projection parameters
        let top = self.area.max.y;
        let bottom = self.area.min.y;
        let right = self.area.max.x;
        let left = self.area.min.x;

        // Calculate scaling factors
        let scale_w = (right - left) / full_width;
        let scale_h = (top - bottom) / full_height;

        // Calculate the new orthographic bounds
        let left_prime = left + scale_w * offset_x;
        let right_prime = left_prime + scale_w * sub_width;
        let top_prime = top - scale_h * offset_y;
        let bottom_prime = top_prime - scale_h * sub_height;

        Mat4::orthographic_rh(
            left_prime,
            right_prime,
            bottom_prime,
            top_prime,
            // NOTE: near and far are swapped to invert the depth range from [0,1] to [1,0]
            // This is for interoperability with pipelines using infinite reverse perspective projections.
            self.far,
            self.near,
        )
    }

    fn update(&mut self, width: f32, height: f32) {
        let (projection_width, projection_height) = match self.scaling_mode {
            ScalingMode::WindowSize(pixel_scale) => (width / pixel_scale, height / pixel_scale),
            ScalingMode::AutoMin {
                min_width,
                min_height,
            } => {
                // Compare Pixels of current width and minimal height and Pixels of minimal width with current height.
                // Then use bigger (min_height when true) as what it refers to (height when true) and calculate rest so it can't get under minimum.
                if width * min_height > min_width * height {
                    (width * min_height / height, min_height)
                } else {
                    (min_width, height * min_width / width)
                }
            }
            ScalingMode::AutoMax {
                max_width,
                max_height,
            } => {
                // Compare Pixels of current width and maximal height and Pixels of maximal width with current height.
                // Then use smaller (max_height when true) as what it refers to (height when true) and calculate rest so it can't get over maximum.
                if width * max_height < max_width * height {
                    (width * max_height / height, max_height)
                } else {
                    (max_width, height * max_width / width)
                }
            }
            ScalingMode::FixedVertical(viewport_height) => {
                (width * viewport_height / height, viewport_height)
            }
            ScalingMode::FixedHorizontal(viewport_width) => {
                (viewport_width, height * viewport_width / width)
            }
            ScalingMode::Fixed { width, height } => (width, height),
        };

        let mut origin_x = projection_width * self.viewport_origin.x;
        let mut origin_y = projection_height * self.viewport_origin.y;

        // If projection is based on window pixels,
        // ensure we don't end up with fractional pixels!
        if let ScalingMode::WindowSize(pixel_scale) = self.scaling_mode {
            // round to nearest multiple of `pixel_scale`
            origin_x = (origin_x * pixel_scale).round() / pixel_scale;
            origin_y = (origin_y * pixel_scale).round() / pixel_scale;
        }

        self.area = Rect::new(
            -origin_x,
            -origin_y,
            projection_width - origin_x,
            projection_height - origin_y,
        );
    }

    fn far(&self) -> f32 {
        self.far
    }

    fn get_frustum_corners(&self, z_near: f32, z_far: f32) -> [Vec3A; 8] {
        let area = self.area;
        // NOTE: These vertices are in the specific order required by [`calculate_cascade`].
        [
            Vec3A::new(area.max.x, area.min.y, z_near), // bottom right
            Vec3A::new(area.max.x, area.max.y, z_near), // top right
            Vec3A::new(area.min.x, area.max.y, z_near), // top left
            Vec3A::new(area.min.x, area.min.y, z_near), // bottom left
            Vec3A::new(area.max.x, area.min.y, z_far),  // bottom right
            Vec3A::new(area.max.x, area.max.y, z_far),  // top right
            Vec3A::new(area.min.x, area.max.y, z_far),  // top left
            Vec3A::new(area.min.x, area.min.y, z_far),  // bottom left
        ]
    }
}

impl FromWorld for OrthographicProjection {
    fn from_world(_world: &mut World) -> Self {
        OrthographicProjection::default_3d()
    }
}

impl OrthographicProjection {
    /// Returns the default orthographic projection for a 2D context.
    ///
    /// The near plane is set to a negative value so that the camera can still
    /// render the scene when using positive z coordinates to order foreground elements.
    pub fn default_2d() -> Self {
        OrthographicProjection {
            near: -1000.0,
            ..OrthographicProjection::default_3d()
        }
    }

    /// Returns the default orthographic projection for a 3D context.
    ///
    /// The near plane is set to 0.0 so that the camera doesn't render
    /// objects that are behind it.
    pub fn default_3d() -> Self {
        OrthographicProjection {
            near: 0.0,
            far: 1000.0,
            viewport_origin: Vec2::new(0.5, 0.5),
            scaling_mode: ScalingMode::WindowSize(1.0),
            area: Rect::new(-1.0, -1.0, 1.0, 1.0),
        }
    }
}
