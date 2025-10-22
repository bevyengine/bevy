use core::fmt::Debug;
use core::ops::{Deref, DerefMut};

use crate::{primitives::Frustum, visibility::VisibilitySystems};
use bevy_app::{App, Plugin, PostUpdate};
use bevy_ecs::prelude::*;
use bevy_math::{ops, AspectRatio, Mat4, Rect, Vec2, Vec3A, Vec4};
use bevy_reflect::{std_traits::ReflectDefault, Reflect, ReflectDeserialize, ReflectSerialize};
use bevy_transform::{components::GlobalTransform, TransformSystems};
use derive_more::derive::From;
use serde::{Deserialize, Serialize};

/// Adds [`Camera`](crate::camera::Camera) driver systems for a given projection type.
///
/// If you are using `bevy_pbr`, then you need to add `PbrProjectionPlugin` along with this.
#[derive(Default)]
pub struct CameraProjectionPlugin;

impl Plugin for CameraProjectionPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            crate::visibility::update_frusta
                .in_set(VisibilitySystems::UpdateFrusta)
                .after(TransformSystems::Propagate),
        );
    }
}

/// Describes a type that can generate a projection matrix, allowing it to be added to a
/// [`Camera`]'s [`Projection`] component.
///
/// Once implemented, the projection can be added to a camera using [`Projection::custom`].
///
/// The projection will be automatically updated as the render area is resized. This is useful when,
/// for example, a projection type has a field like `fov` that should change when the window width
/// is changed but not when the height changes.
///
/// This trait is implemented by bevy's built-in projections [`PerspectiveProjection`] and
/// [`OrthographicProjection`].
///
/// [`Camera`]: crate::camera::Camera
pub trait CameraProjection {
    /// Generate the projection matrix.
    fn get_clip_from_view(&self) -> Mat4;

    /// Generate the projection matrix for a [`SubCameraView`](super::SubCameraView).
    fn get_clip_from_view_for_sub(&self, sub_view: &super::SubCameraView) -> Mat4;

    /// When the area this camera renders to changes dimensions, this method will be automatically
    /// called. Use this to update any projection properties that depend on the aspect ratio or
    /// dimensions of the render area.
    fn update(&mut self, width: f32, height: f32);

    /// The far plane distance of the projection.
    fn far(&self) -> f32;

    /// The eight corners of the camera frustum, as defined by this projection.
    ///
    /// The corners should be provided in the following order: first the bottom right, top right,
    /// top left, bottom left for the near plane, then similar for the far plane.
    // TODO: This seems somewhat redundant with `compute_frustum`, and similarly should be possible
    // to compute with a default impl.
    fn get_frustum_corners(&self, z_near: f32, z_far: f32) -> [Vec3A; 8];

    /// Compute camera frustum for camera with given projection and transform.
    ///
    /// This code is called by [`update_frusta`](crate::visibility::update_frusta) system
    /// for each camera to update its frustum.
    fn compute_frustum(&self, camera_transform: &GlobalTransform) -> Frustum {
        let clip_from_world = self.get_clip_from_view() * camera_transform.affine().inverse();
        Frustum::from_clip_from_world_custom_far(
            &clip_from_world,
            &camera_transform.translation(),
            &camera_transform.back(),
            self.far(),
        )
    }
}

mod sealed {
    use super::CameraProjection;

    /// A wrapper trait to make it possible to implement Clone for boxed [`CameraProjection`](`super::CameraProjection`)
    /// trait objects, without breaking object safety rules by making it `Sized`. Additional bounds
    /// are included for downcasting, and fulfilling the trait bounds on `Projection`.
    pub trait DynCameraProjection:
        CameraProjection + core::fmt::Debug + Send + Sync + downcast_rs::Downcast
    {
        fn clone_box(&self) -> Box<dyn DynCameraProjection>;
    }

    downcast_rs::impl_downcast!(DynCameraProjection);

    impl<T> DynCameraProjection for T
    where
        T: 'static + CameraProjection + core::fmt::Debug + Send + Sync + Clone,
    {
        fn clone_box(&self) -> Box<dyn DynCameraProjection> {
            Box::new(self.clone())
        }
    }
}

/// Holds a dynamic [`CameraProjection`] trait object. Use [`Projection::custom()`] to construct a
/// custom projection.
///
/// The contained dynamic object can be downcast into a static type using [`CustomProjection::get`].
#[derive(Debug, Reflect)]
#[reflect(Default, Clone)]
pub struct CustomProjection {
    #[reflect(ignore)]
    dyn_projection: Box<dyn sealed::DynCameraProjection>,
}

impl Default for CustomProjection {
    fn default() -> Self {
        Self {
            dyn_projection: Box::new(PerspectiveProjection::default()),
        }
    }
}

impl Clone for CustomProjection {
    fn clone(&self) -> Self {
        Self {
            dyn_projection: self.dyn_projection.clone_box(),
        }
    }
}

impl CustomProjection {
    /// Returns a reference to the [`CameraProjection`] `P`.
    ///
    /// Returns `None` if this dynamic object is not a projection of type `P`.
    ///
    /// ```
    /// # use bevy_camera::{Projection, PerspectiveProjection};
    /// // For simplicity's sake, use perspective as a custom projection:
    /// let projection = Projection::custom(PerspectiveProjection::default());
    /// let Projection::Custom(custom) = projection else { return };
    ///
    /// // At this point the projection type is erased.
    /// // We can use `get()` if we know what kind of projection we have.
    /// let perspective = custom.get::<PerspectiveProjection>().unwrap();
    ///
    /// assert_eq!(perspective.fov, PerspectiveProjection::default().fov);
    /// ```
    pub fn get<P>(&self) -> Option<&P>
    where
        P: CameraProjection + Debug + Send + Sync + Clone + 'static,
    {
        self.dyn_projection.downcast_ref()
    }

    /// Returns a mutable  reference to the [`CameraProjection`] `P`.
    ///
    /// Returns `None` if this dynamic object is not a projection of type `P`.
    ///
    /// ```
    /// # use bevy_camera::{Projection, PerspectiveProjection};
    /// // For simplicity's sake, use perspective as a custom projection:
    /// let mut projection = Projection::custom(PerspectiveProjection::default());
    /// let Projection::Custom(mut custom) = projection else { return };
    ///
    /// // At this point the projection type is erased.
    /// // We can use `get_mut()` if we know what kind of projection we have.
    /// let perspective = custom.get_mut::<PerspectiveProjection>().unwrap();
    ///
    /// assert_eq!(perspective.fov, PerspectiveProjection::default().fov);
    /// perspective.fov = 1.0;
    /// ```
    pub fn get_mut<P>(&mut self) -> Option<&mut P>
    where
        P: CameraProjection + Debug + Send + Sync + Clone + 'static,
    {
        self.dyn_projection.downcast_mut()
    }
}

impl Deref for CustomProjection {
    type Target = dyn CameraProjection;

    fn deref(&self) -> &Self::Target {
        self.dyn_projection.as_ref()
    }
}

impl DerefMut for CustomProjection {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.dyn_projection.as_mut()
    }
}

/// Component that defines how to compute a [`Camera`]'s projection matrix.
///
/// Common projections, like perspective and orthographic, are provided out of the box to handle the
/// majority of use cases. Custom projections can be added using the [`CameraProjection`] trait and
/// the [`Projection::custom`] constructor.
///
/// ## What's a projection?
///
/// A camera projection essentially describes how 3d points from the point of view of a camera are
/// projected onto a 2d screen. This is where properties like a camera's field of view are defined.
/// More specifically, a projection is a 4x4 matrix that transforms points from view space (the
/// point of view of the camera) into clip space. Clip space is almost, but not quite, equivalent to
/// the rectangle that is rendered to your screen, with a depth axis. Any points that land outside
/// the bounds of this cuboid are "clipped" and not rendered.
///
/// You can also think of the projection as the thing that describes the shape of a camera's
/// frustum: the volume in 3d space that is visible to a camera.
///
/// [`Camera`]: crate::camera::Camera
#[derive(Component, Debug, Clone, Reflect, From)]
#[reflect(Component, Default, Debug, Clone)]
pub enum Projection {
    Perspective(PerspectiveProjection),
    Orthographic(OrthographicProjection),
    Custom(CustomProjection),
}

impl Projection {
    /// Construct a new custom camera projection from a type that implements [`CameraProjection`].
    pub fn custom<P>(projection: P) -> Self
    where
        // Implementation note: pushing these trait bounds all the way out to this function makes
        // errors nice for users. If a trait is missing, they will get a helpful error telling them
        // that, say, the `Debug` implementation is missing. Wrapping these traits behind a super
        // trait or some other indirection will make the errors harder to understand.
        //
        // For example, we don't use the `DynCameraProjection` trait bound, because it is not the
        // trait the user should be implementing - they only need to worry about implementing
        // `CameraProjection`.
        P: CameraProjection + Debug + Send + Sync + Clone + 'static,
    {
        Projection::Custom(CustomProjection {
            dyn_projection: Box::new(projection),
        })
    }

    /// Check if the projection is perspective.
    /// For [`CustomProjection`], this checks if the projection matrix's w-axis's w is 0.0.
    pub fn is_perspective(&self) -> bool {
        match self {
            Projection::Perspective(_) => true,
            Projection::Orthographic(_) => false,
            Projection::Custom(projection) => projection.get_clip_from_view().w_axis.w == 0.0,
        }
    }
}

impl Deref for Projection {
    type Target = dyn CameraProjection;

    fn deref(&self) -> &Self::Target {
        match self {
            Projection::Perspective(projection) => projection,
            Projection::Orthographic(projection) => projection,
            Projection::Custom(projection) => projection.deref(),
        }
    }
}

impl DerefMut for Projection {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            Projection::Perspective(projection) => projection,
            Projection::Orthographic(projection) => projection,
            Projection::Custom(projection) => projection.deref_mut(),
        }
    }
}

impl Default for Projection {
    fn default() -> Self {
        Projection::Perspective(Default::default())
    }
}

/// A 3D camera projection in which distant objects appear smaller than close objects.
#[derive(Debug, Clone, Reflect)]
#[reflect(Default, Debug, Clone)]
pub struct PerspectiveProjection {
    /// The vertical field of view (FOV) in radians.
    ///
    /// Defaults to a value of π/4 radians or 45 degrees.
    pub fov: f32,

    /// The aspect ratio (width divided by height) of the viewing frustum.
    ///
    /// Bevy's `camera_system` automatically updates this value when the aspect ratio
    /// of the associated window changes.
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

        let full_aspect = full_width / full_height;

        // Original frustum parameters
        let top = self.near * ops::tan(0.5 * self.fov);
        let bottom = -top;
        let right = top * full_aspect;
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
/// The effect of these scaling modes are combined with the [`OrthographicProjection::scale`] property.
///
/// For example, if the scaling mode is `ScalingMode::Fixed { width: 100.0, height: 300 }` and the scale is `2.0`,
/// the projection will be 200 world units wide and 600 world units tall.
///
/// # Examples
///
/// Configure the orthographic projection to two world units per window height:
///
/// ```
/// # use bevy_camera::{OrthographicProjection, Projection, ScalingMode};
/// let projection = Projection::Orthographic(OrthographicProjection {
///    scaling_mode: ScalingMode::FixedVertical { viewport_height: 2.0 },
///    ..OrthographicProjection::default_2d()
/// });
/// ```
#[derive(Default, Debug, Clone, Copy, Reflect, Serialize, Deserialize)]
#[reflect(Serialize, Deserialize, Default, Clone)]
pub enum ScalingMode {
    /// Match the viewport size.
    ///
    /// With a scale of 1, lengths in world units will map 1:1 with the number of pixels used to render it.
    /// For example, if we have a 64x64 sprite with a [`Transform::scale`](bevy_transform::prelude::Transform) of 1.0,
    /// no custom size and no inherited scale, the sprite will be 64 world units wide and 64 world units tall.
    /// When rendered with [`OrthographicProjection::scaling_mode`] set to `WindowSize` when the window scale factor is 1
    /// the sprite will be rendered at 64 pixels wide and 64 pixels tall.
    ///
    /// Changing any of these properties will multiplicatively affect the final size.
    #[default]
    WindowSize,
    /// Manually specify the projection's size, ignoring window resizing. The image will stretch.
    ///
    /// Arguments describe the area of the world that is shown (in world units).
    Fixed { width: f32, height: f32 },
    /// Keeping the aspect ratio while the axes can't be smaller than given minimum.
    ///
    /// Arguments are in world units.
    AutoMin { min_width: f32, min_height: f32 },
    /// Keeping the aspect ratio while the axes can't be bigger than given maximum.
    ///
    /// Arguments are in world units.
    AutoMax { max_width: f32, max_height: f32 },
    /// Keep the projection's height constant; width will be adjusted to match aspect ratio.
    ///
    /// The argument is the desired height of the projection in world units.
    FixedVertical { viewport_height: f32 },
    /// Keep the projection's width constant; height will be adjusted to match aspect ratio.
    ///
    /// The argument is the desired width of the projection in world units.
    FixedHorizontal { viewport_width: f32 },
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
/// # use bevy_camera::{OrthographicProjection, Projection, ScalingMode};
/// let projection = Projection::Orthographic(OrthographicProjection {
///     scaling_mode: ScalingMode::WindowSize,
///     scale: 0.01,
///     ..OrthographicProjection::default_2d()
/// });
/// ```
#[derive(Debug, Clone, Reflect)]
#[reflect(Debug, FromWorld, Clone)]
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
    /// Defaults to [`ScalingMode::WindowSize`],
    /// and works in concert with [`OrthographicProjection::scale`] to determine the final effect.
    ///
    /// For simplicity, zooming should be done by changing [`OrthographicProjection::scale`],
    /// rather than changing the parameters of the scaling mode.
    pub scaling_mode: ScalingMode,
    /// Scales the projection.
    ///
    /// As scale increases, the apparent size of objects decreases, and vice versa.
    ///
    /// Note: scaling can be set by [`scaling_mode`](Self::scaling_mode) as well.
    /// This parameter scales on top of that.
    ///
    /// This property is particularly useful in implementing zoom functionality.
    ///
    /// Defaults to `1.0`, which under standard settings corresponds to a 1:1 mapping of world units to rendered pixels.
    /// See [`ScalingMode::WindowSize`] for more information.
    pub scale: f32,
    /// The area that the projection covers relative to `viewport_origin`.
    ///
    /// Bevy's `camera_system` automatically
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

        let full_aspect = full_width / full_height;

        // Base the vertical size on self.area and adjust the horizontal size
        let top = self.area.max.y;
        let bottom = self.area.min.y;
        let ortho_height = top - bottom;
        let ortho_width = ortho_height * full_aspect;

        // Center the orthographic area horizontally
        let center_x = (self.area.max.x + self.area.min.x) / 2.0;
        let left = center_x - ortho_width / 2.0;
        let right = center_x + ortho_width / 2.0;

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
            ScalingMode::WindowSize => (width, height),
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
            ScalingMode::FixedVertical { viewport_height } => {
                (width * viewport_height / height, viewport_height)
            }
            ScalingMode::FixedHorizontal { viewport_width } => {
                (viewport_width, height * viewport_width / width)
            }
            ScalingMode::Fixed { width, height } => (width, height),
        };

        let origin_x = projection_width * self.viewport_origin.x;
        let origin_y = projection_height * self.viewport_origin.y;

        self.area = Rect::new(
            self.scale * -origin_x,
            self.scale * -origin_y,
            self.scale * (projection_width - origin_x),
            self.scale * (projection_height - origin_y),
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
            scale: 1.0,
            near: 0.0,
            far: 1000.0,
            viewport_origin: Vec2::new(0.5, 0.5),
            scaling_mode: ScalingMode::WindowSize,
            area: Rect::new(-1.0, -1.0, 1.0, 1.0),
        }
    }
}
