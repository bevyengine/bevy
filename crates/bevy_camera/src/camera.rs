use crate::primitives::Frustum;

use super::{
    visibility::{Visibility, VisibleEntities},
    ClearColorConfig,
};
use bevy_asset::Handle;
use bevy_derive::Deref;
use bevy_ecs::{component::Component, entity::Entity, reflect::ReflectComponent};
use bevy_image::Image;
use bevy_math::{ops, Dir3, FloatOrd, Mat4, Ray3d, Rect, URect, UVec2, Vec2, Vec3};
use bevy_reflect::prelude::*;
use bevy_transform::components::{GlobalTransform, Transform};
use bevy_window::{NormalizedWindowRef, WindowRef};
use core::ops::Range;
use derive_more::derive::From;
use thiserror::Error;
use wgpu_types::{BlendState, TextureUsages};

/// Render viewport configuration for the [`Camera`] component.
///
/// The viewport defines the area on the render target to which the camera renders its image.
/// You can overlay multiple cameras in a single window using viewports to create effects like
/// split screen, minimaps, and character viewers.
#[derive(Reflect, Debug, Clone)]
#[reflect(Default, Clone)]
pub struct Viewport {
    /// The physical position to render this viewport to within the [`RenderTarget`] of this [`Camera`].
    /// (0,0) corresponds to the top-left corner
    pub physical_position: UVec2,
    /// The physical size of the viewport rectangle to render to within the [`RenderTarget`] of this [`Camera`].
    /// The origin of the rectangle is in the top-left corner.
    pub physical_size: UVec2,
    /// The minimum and maximum depth to render (on a scale from 0.0 to 1.0).
    pub depth: Range<f32>,
}

impl Default for Viewport {
    fn default() -> Self {
        Self {
            physical_position: Default::default(),
            physical_size: UVec2::new(1, 1),
            depth: 0.0..1.0,
        }
    }
}

impl Viewport {
    /// Cut the viewport rectangle so that it lies inside a rectangle of the
    /// given size.
    ///
    /// If either of the viewport's position coordinates lies outside the given
    /// dimensions, it will be moved just inside first. If either of the given
    /// dimensions is zero, the position and size of the viewport rectangle will
    /// both be set to zero in that dimension.
    pub fn clamp_to_size(&mut self, size: UVec2) {
        // If the origin of the viewport rect is outside, then adjust so that
        // it's just barely inside. Then, cut off the part that is outside.
        if self.physical_size.x + self.physical_position.x > size.x {
            if self.physical_position.x < size.x {
                self.physical_size.x = size.x - self.physical_position.x;
            } else if size.x > 0 {
                self.physical_position.x = size.x - 1;
                self.physical_size.x = 1;
            } else {
                self.physical_position.x = 0;
                self.physical_size.x = 0;
            }
        }
        if self.physical_size.y + self.physical_position.y > size.y {
            if self.physical_position.y < size.y {
                self.physical_size.y = size.y - self.physical_position.y;
            } else if size.y > 0 {
                self.physical_position.y = size.y - 1;
                self.physical_size.y = 1;
            } else {
                self.physical_position.y = 0;
                self.physical_size.y = 0;
            }
        }
    }

    pub fn from_viewport_and_override(
        viewport: Option<&Self>,
        main_pass_resolution_override: Option<&MainPassResolutionOverride>,
    ) -> Option<Self> {
        let mut viewport = viewport.cloned();

        if let Some(override_size) = main_pass_resolution_override {
            if viewport.is_none() {
                viewport = Some(Viewport::default());
            }

            viewport.as_mut().unwrap().physical_size = **override_size;
        }

        viewport
    }
}

/// Override the resolution a 3d camera's main pass is rendered at.
///
/// Does not affect post processing.
///
/// ## Usage
///
/// * Insert this component on a 3d camera entity in the render world.
/// * The resolution override must be smaller than the camera's viewport size.
/// * The resolution override is specified in physical pixels.
/// * In shaders, use `View::main_pass_viewport` instead of `View::viewport`.
#[derive(Component, Reflect, Deref, Debug)]
#[reflect(Component)]
pub struct MainPassResolutionOverride(pub UVec2);

/// Settings to define a camera sub view.
///
/// When [`Camera::sub_camera_view`] is `Some`, only the sub-section of the
/// image defined by `size` and `offset` (relative to the `full_size` of the
/// whole image) is projected to the cameras viewport.
///
/// Take the example of the following multi-monitor setup:
/// ```css
/// ┌───┬───┐
/// │ A │ B │
/// ├───┼───┤
/// │ C │ D │
/// └───┴───┘
/// ```
/// If each monitor is 1920x1080, the whole image will have a resolution of
/// 3840x2160. For each monitor we can use a single camera with a viewport of
/// the same size as the monitor it corresponds to. To ensure that the image is
/// cohesive, we can use a different sub view on each camera:
/// - Camera A: `full_size` = 3840x2160, `size` = 1920x1080, `offset` = 0,0
/// - Camera B: `full_size` = 3840x2160, `size` = 1920x1080, `offset` = 1920,0
/// - Camera C: `full_size` = 3840x2160, `size` = 1920x1080, `offset` = 0,1080
/// - Camera D: `full_size` = 3840x2160, `size` = 1920x1080, `offset` =
///   1920,1080
///
/// However since only the ratio between the values is important, they could all
/// be divided by 120 and still produce the same image. Camera D would for
/// example have the following values:
/// `full_size` = 32x18, `size` = 16x9, `offset` = 16,9
#[derive(Debug, Clone, Copy, Reflect, PartialEq)]
#[reflect(Clone, PartialEq, Default)]
pub struct SubCameraView {
    /// Size of the entire camera view
    pub full_size: UVec2,
    /// Offset of the sub camera
    pub offset: Vec2,
    /// Size of the sub camera
    pub size: UVec2,
}

impl Default for SubCameraView {
    fn default() -> Self {
        Self {
            full_size: UVec2::new(1, 1),
            offset: Vec2::new(0., 0.),
            size: UVec2::new(1, 1),
        }
    }
}

/// Information about the current [`RenderTarget`].
#[derive(Default, Debug, Clone)]
pub struct RenderTargetInfo {
    /// The physical size of this render target (in physical pixels, ignoring scale factor).
    pub physical_size: UVec2,
    /// The scale factor of this render target.
    ///
    /// When rendering to a window, typically it is a value greater or equal than 1.0,
    /// representing the ratio between the size of the window in physical pixels and the logical size of the window.
    pub scale_factor: f32,
}

/// Holds internally computed [`Camera`] values.
#[derive(Default, Debug, Clone)]
pub struct ComputedCameraValues {
    pub clip_from_view: Mat4,
    pub target_info: Option<RenderTargetInfo>,
    // size of the `Viewport`
    pub old_viewport_size: Option<UVec2>,
    pub old_sub_camera_view: Option<SubCameraView>,
}

/// How much energy a [`Camera3d`](crate::Camera3d) absorbs from incoming light.
///
/// <https://en.wikipedia.org/wiki/Exposure_(photography)>
#[derive(Component, Clone, Copy, Reflect)]
#[reflect(opaque)]
#[reflect(Component, Default, Clone)]
pub struct Exposure {
    /// <https://en.wikipedia.org/wiki/Exposure_value#Tabulated_exposure_values>
    pub ev100: f32,
}

impl Exposure {
    pub const SUNLIGHT: Self = Self {
        ev100: Self::EV100_SUNLIGHT,
    };
    pub const OVERCAST: Self = Self {
        ev100: Self::EV100_OVERCAST,
    };
    pub const INDOOR: Self = Self {
        ev100: Self::EV100_INDOOR,
    };
    /// This value was calibrated to match Blender's implicit/default exposure as closely as possible.
    /// It also happens to be a reasonable default.
    ///
    /// See <https://github.com/bevyengine/bevy/issues/11577> for details.
    pub const BLENDER: Self = Self {
        ev100: Self::EV100_BLENDER,
    };

    pub const EV100_SUNLIGHT: f32 = 15.0;
    pub const EV100_OVERCAST: f32 = 12.0;
    pub const EV100_INDOOR: f32 = 7.0;

    /// This value was calibrated to match Blender's implicit/default exposure as closely as possible.
    /// It also happens to be a reasonable default.
    ///
    /// See <https://github.com/bevyengine/bevy/issues/11577> for details.
    pub const EV100_BLENDER: f32 = 9.7;

    pub fn from_physical_camera(physical_camera_parameters: PhysicalCameraParameters) -> Self {
        Self {
            ev100: physical_camera_parameters.ev100(),
        }
    }

    /// Converts EV100 values to exposure values.
    /// <https://google.github.io/filament/Filament.md.html#imagingpipeline/physicallybasedcamera/exposure>
    #[inline]
    pub fn exposure(&self) -> f32 {
        ops::exp2(-self.ev100) / 1.2
    }
}

impl Default for Exposure {
    fn default() -> Self {
        Self::BLENDER
    }
}

/// Parameters based on physical camera characteristics for calculating EV100
/// values for use with [`Exposure`]. This is also used for depth of field.
#[derive(Clone, Copy)]
pub struct PhysicalCameraParameters {
    /// <https://en.wikipedia.org/wiki/F-number>
    pub aperture_f_stops: f32,
    /// <https://en.wikipedia.org/wiki/Shutter_speed>
    pub shutter_speed_s: f32,
    /// <https://en.wikipedia.org/wiki/Film_speed>
    pub sensitivity_iso: f32,
    /// The height of the [image sensor format] in meters.
    ///
    /// Focal length is derived from the FOV and this value. The default is
    /// 18.66mm, matching the [Super 35] format, which is popular in cinema.
    ///
    /// [image sensor format]: https://en.wikipedia.org/wiki/Image_sensor_format
    ///
    /// [Super 35]: https://en.wikipedia.org/wiki/Super_35
    pub sensor_height: f32,
}

impl PhysicalCameraParameters {
    /// Calculate the [EV100](https://en.wikipedia.org/wiki/Exposure_value).
    pub fn ev100(&self) -> f32 {
        ops::log2(
            self.aperture_f_stops * self.aperture_f_stops * 100.0
                / (self.shutter_speed_s * self.sensitivity_iso),
        )
    }
}

impl Default for PhysicalCameraParameters {
    fn default() -> Self {
        Self {
            aperture_f_stops: 1.0,
            shutter_speed_s: 1.0 / 125.0,
            sensitivity_iso: 100.0,
            sensor_height: 0.01866,
        }
    }
}

/// Error returned when a conversion between world-space and viewport-space coordinates fails.
///
/// See [`world_to_viewport`][Camera::world_to_viewport] and [`viewport_to_world`][Camera::viewport_to_world].
#[derive(Debug, Eq, PartialEq, Copy, Clone, Error)]
pub enum ViewportConversionError {
    /// The pre-computed size of the viewport was not available.
    ///
    /// This may be because the `Camera` was just created and `camera_system` has not been executed
    /// yet, or because the [`RenderTarget`] is misconfigured in one of the following ways:
    ///   - it references the [`PrimaryWindow`](RenderTarget::Window) when there is none,
    ///   - it references a [`Window`](RenderTarget::Window) entity that doesn't exist or doesn't actually have a `Window` component,
    ///   - it references an [`Image`](RenderTarget::Image) that doesn't exist (invalid handle),
    ///   - it references a [`TextureView`](RenderTarget::TextureView) that doesn't exist (invalid handle).
    #[error("pre-computed size of viewport not available")]
    NoViewportSize,
    /// The computed coordinate was beyond the `Camera`'s near plane.
    ///
    /// Only applicable when converting from world-space to viewport-space.
    #[error("computed coordinate beyond `Camera`'s near plane")]
    PastNearPlane,
    /// The computed coordinate was beyond the `Camera`'s far plane.
    ///
    /// Only applicable when converting from world-space to viewport-space.
    #[error("computed coordinate beyond `Camera`'s far plane")]
    PastFarPlane,
    /// The Normalized Device Coordinates could not be computed because the `camera_transform`, the
    /// `world_position`, or the projection matrix defined by [`Projection`](super::projection::Projection)
    /// contained `NAN` (see [`world_to_ndc`][Camera::world_to_ndc] and [`ndc_to_world`][Camera::ndc_to_world]).
    #[error("found NaN while computing NDC")]
    InvalidData,
}

/// The defining [`Component`] for camera entities,
/// storing information about how and what to render through this camera.
///
/// The [`Camera`] component is added to an entity to define the properties of the viewpoint from
/// which rendering occurs. It defines the position of the view to render, the projection method
/// to transform the 3D objects into a 2D image, as well as the render target into which that image
/// is produced.
///
/// Note that a [`Camera`] needs a `CameraRenderGraph` to render anything.
/// This is typically provided by adding a [`Camera2d`] or [`Camera3d`] component,
/// but custom render graphs can also be defined. Inserting a [`Camera`] with no render
/// graph will emit an error at runtime.
///
/// [`Camera2d`]: crate::Camera2d
/// [`Camera3d`]: crate::Camera3d
#[derive(Component, Debug, Reflect, Clone)]
#[reflect(Component, Default, Debug, Clone)]
#[require(
    Frustum,
    CameraMainTextureUsages,
    VisibleEntities,
    Transform,
    Visibility
)]
pub struct Camera {
    /// If set, this camera will render to the given [`Viewport`] rectangle within the configured [`RenderTarget`].
    pub viewport: Option<Viewport>,
    /// Cameras with a higher order are rendered later, and thus on top of lower order cameras.
    pub order: isize,
    /// If this is set to `true`, this camera will be rendered to its specified [`RenderTarget`]. If `false`, this
    /// camera will not be rendered.
    pub is_active: bool,
    /// Computed values for this camera, such as the projection matrix and the render target size.
    #[reflect(ignore, clone)]
    pub computed: ComputedCameraValues,
    /// The "target" that this camera will render to.
    pub target: RenderTarget,
    // todo: reflect this when #6042 lands
    /// The [`CameraOutputMode`] for this camera.
    #[reflect(ignore, clone)]
    pub output_mode: CameraOutputMode,
    /// If this is enabled, a previous camera exists that shares this camera's render target, and this camera has MSAA enabled, then the previous camera's
    /// outputs will be written to the intermediate multi-sampled render target textures for this camera. This enables cameras with MSAA enabled to
    /// "write their results on top" of previous camera results, and include them as a part of their render results. This is enabled by default to ensure
    /// cameras with MSAA enabled layer their results in the same way as cameras without MSAA enabled by default.
    pub msaa_writeback: bool,
    /// The clear color operation to perform on the render target.
    pub clear_color: ClearColorConfig,
    /// If set, this camera will be a sub camera of a large view, defined by a [`SubCameraView`].
    pub sub_camera_view: Option<SubCameraView>,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            is_active: true,
            order: 0,
            viewport: None,
            computed: Default::default(),
            target: Default::default(),
            output_mode: Default::default(),
            msaa_writeback: true,
            clear_color: Default::default(),
            sub_camera_view: None,
        }
    }
}

impl Camera {
    /// Converts a physical size in this `Camera` to a logical size.
    #[inline]
    pub fn to_logical(&self, physical_size: UVec2) -> Option<Vec2> {
        let scale = self.computed.target_info.as_ref()?.scale_factor;
        Some(physical_size.as_vec2() / scale)
    }

    /// The rendered physical bounds [`URect`] of the camera. If the `viewport` field is
    /// set to [`Some`], this will be the rect of that custom viewport. Otherwise it will default to
    /// the full physical rect of the current [`RenderTarget`].
    #[inline]
    pub fn physical_viewport_rect(&self) -> Option<URect> {
        let min = self
            .viewport
            .as_ref()
            .map(|v| v.physical_position)
            .unwrap_or(UVec2::ZERO);
        let max = min + self.physical_viewport_size()?;
        Some(URect { min, max })
    }

    /// The rendered logical bounds [`Rect`] of the camera. If the `viewport` field is set to
    /// [`Some`], this will be the rect of that custom viewport. Otherwise it will default to the
    /// full logical rect of the current [`RenderTarget`].
    #[inline]
    pub fn logical_viewport_rect(&self) -> Option<Rect> {
        let URect { min, max } = self.physical_viewport_rect()?;
        Some(Rect {
            min: self.to_logical(min)?,
            max: self.to_logical(max)?,
        })
    }

    /// The logical size of this camera's viewport. If the `viewport` field is set to [`Some`], this
    /// will be the size of that custom viewport. Otherwise it will default to the full logical size
    /// of the current [`RenderTarget`].
    ///  For logic that requires the full logical size of the
    /// [`RenderTarget`], prefer [`Camera::logical_target_size`].
    ///
    /// Returns `None` if either:
    /// - the function is called just after the `Camera` is created, before `camera_system` is executed,
    /// - the [`RenderTarget`] isn't correctly set:
    ///   - it references the [`PrimaryWindow`](RenderTarget::Window) when there is none,
    ///   - it references a [`Window`](RenderTarget::Window) entity that doesn't exist or doesn't actually have a `Window` component,
    ///   - it references an [`Image`](RenderTarget::Image) that doesn't exist (invalid handle),
    ///   - it references a [`TextureView`](RenderTarget::TextureView) that doesn't exist (invalid handle).
    #[inline]
    pub fn logical_viewport_size(&self) -> Option<Vec2> {
        self.viewport
            .as_ref()
            .and_then(|v| self.to_logical(v.physical_size))
            .or_else(|| self.logical_target_size())
    }

    /// The physical size of this camera's viewport (in physical pixels).
    /// If the `viewport` field is set to [`Some`], this
    /// will be the size of that custom viewport. Otherwise it will default to the full physical size of
    /// the current [`RenderTarget`].
    /// For logic that requires the full physical size of the [`RenderTarget`], prefer [`Camera::physical_target_size`].
    #[inline]
    pub fn physical_viewport_size(&self) -> Option<UVec2> {
        self.viewport
            .as_ref()
            .map(|v| v.physical_size)
            .or_else(|| self.physical_target_size())
    }

    /// The full logical size of this camera's [`RenderTarget`], ignoring custom `viewport` configuration.
    /// Note that if the `viewport` field is [`Some`], this will not represent the size of the rendered area.
    /// For logic that requires the size of the actually rendered area, prefer [`Camera::logical_viewport_size`].
    #[inline]
    pub fn logical_target_size(&self) -> Option<Vec2> {
        self.computed
            .target_info
            .as_ref()
            .and_then(|t| self.to_logical(t.physical_size))
    }

    /// The full physical size of this camera's [`RenderTarget`] (in physical pixels),
    /// ignoring custom `viewport` configuration.
    /// Note that if the `viewport` field is [`Some`], this will not represent the size of the rendered area.
    /// For logic that requires the size of the actually rendered area, prefer [`Camera::physical_viewport_size`].
    #[inline]
    pub fn physical_target_size(&self) -> Option<UVec2> {
        self.computed.target_info.as_ref().map(|t| t.physical_size)
    }

    #[inline]
    pub fn target_scaling_factor(&self) -> Option<f32> {
        self.computed
            .target_info
            .as_ref()
            .map(|t: &RenderTargetInfo| t.scale_factor)
    }

    /// The projection matrix computed using this camera's [`Projection`](super::projection::Projection).
    #[inline]
    pub fn clip_from_view(&self) -> Mat4 {
        self.computed.clip_from_view
    }

    /// Given a position in world space, use the camera to compute the viewport-space coordinates.
    ///
    /// To get the coordinates in Normalized Device Coordinates, you should use
    /// [`world_to_ndc`](Self::world_to_ndc).
    ///
    /// # Panics
    ///
    /// Will panic if `glam_assert` is enabled and the `camera_transform` contains `NAN`
    /// (see [`world_to_ndc`][Self::world_to_ndc]).
    #[doc(alias = "world_to_screen")]
    pub fn world_to_viewport(
        &self,
        camera_transform: &GlobalTransform,
        world_position: Vec3,
    ) -> Result<Vec2, ViewportConversionError> {
        let target_rect = self
            .logical_viewport_rect()
            .ok_or(ViewportConversionError::NoViewportSize)?;
        let mut ndc_space_coords = self
            .world_to_ndc(camera_transform, world_position)
            .ok_or(ViewportConversionError::InvalidData)?;
        // NDC z-values outside of 0 < z < 1 are outside the (implicit) camera frustum and are thus not in viewport-space
        if ndc_space_coords.z < 0.0 {
            return Err(ViewportConversionError::PastNearPlane);
        }
        if ndc_space_coords.z > 1.0 {
            return Err(ViewportConversionError::PastFarPlane);
        }

        // Flip the Y co-ordinate origin from the bottom to the top.
        ndc_space_coords.y = -ndc_space_coords.y;

        // Once in NDC space, we can discard the z element and map x/y to the viewport rect
        let viewport_position =
            (ndc_space_coords.truncate() + Vec2::ONE) / 2.0 * target_rect.size() + target_rect.min;
        Ok(viewport_position)
    }

    /// Given a position in world space, use the camera to compute the viewport-space coordinates and depth.
    ///
    /// To get the coordinates in Normalized Device Coordinates, you should use
    /// [`world_to_ndc`](Self::world_to_ndc).
    ///
    /// # Panics
    ///
    /// Will panic if `glam_assert` is enabled and the `camera_transform` contains `NAN`
    /// (see [`world_to_ndc`][Self::world_to_ndc]).
    #[doc(alias = "world_to_screen_with_depth")]
    pub fn world_to_viewport_with_depth(
        &self,
        camera_transform: &GlobalTransform,
        world_position: Vec3,
    ) -> Result<Vec3, ViewportConversionError> {
        let target_rect = self
            .logical_viewport_rect()
            .ok_or(ViewportConversionError::NoViewportSize)?;
        let mut ndc_space_coords = self
            .world_to_ndc(camera_transform, world_position)
            .ok_or(ViewportConversionError::InvalidData)?;
        // NDC z-values outside of 0 < z < 1 are outside the (implicit) camera frustum and are thus not in viewport-space
        if ndc_space_coords.z < 0.0 {
            return Err(ViewportConversionError::PastNearPlane);
        }
        if ndc_space_coords.z > 1.0 {
            return Err(ViewportConversionError::PastFarPlane);
        }

        // Stretching ndc depth to value via near plane and negating result to be in positive room again.
        let depth = -self.depth_ndc_to_view_z(ndc_space_coords.z);

        // Flip the Y co-ordinate origin from the bottom to the top.
        ndc_space_coords.y = -ndc_space_coords.y;

        // Once in NDC space, we can discard the z element and map x/y to the viewport rect
        let viewport_position =
            (ndc_space_coords.truncate() + Vec2::ONE) / 2.0 * target_rect.size() + target_rect.min;
        Ok(viewport_position.extend(depth))
    }

    /// Returns a ray originating from the camera, that passes through everything beyond `viewport_position`.
    ///
    /// The resulting ray starts on the near plane of the camera.
    ///
    /// If the camera's projection is orthographic the direction of the ray is always equal to `camera_transform.forward()`.
    ///
    /// To get the world space coordinates with Normalized Device Coordinates, you should use
    /// [`ndc_to_world`](Self::ndc_to_world).
    ///
    /// # Example
    /// ```no_run
    /// # use bevy_window::Window;
    /// # use bevy_ecs::prelude::{Single, IntoScheduleConfigs};
    /// # use bevy_transform::prelude::{GlobalTransform, TransformSystems};
    /// # use bevy_camera::Camera;
    /// # use bevy_app::{App, PostUpdate};
    /// #
    /// fn system(camera_query: Single<(&Camera, &GlobalTransform)>, window: Single<&Window>) {
    ///     let (camera, camera_transform) = *camera_query;
    ///
    ///     if let Some(cursor_position) = window.cursor_position()
    ///         // Calculate a ray pointing from the camera into the world based on the cursor's position.
    ///         && let Ok(ray) = camera.viewport_to_world(camera_transform, cursor_position)
    ///     {
    ///         println!("{ray:?}");
    ///     }
    /// }
    ///
    /// # let mut app = App::new();
    /// // Run the system after transform propagation so the camera's global transform is up-to-date.
    /// app.add_systems(PostUpdate, system.after(TransformSystems::Propagate));
    /// ```
    ///
    /// # Panics
    ///
    /// Will panic if the camera's projection matrix is invalid (has a determinant of 0) and
    /// `glam_assert` is enabled (see [`ndc_to_world`](Self::ndc_to_world).
    pub fn viewport_to_world(
        &self,
        camera_transform: &GlobalTransform,
        viewport_position: Vec2,
    ) -> Result<Ray3d, ViewportConversionError> {
        let target_rect = self
            .logical_viewport_rect()
            .ok_or(ViewportConversionError::NoViewportSize)?;
        let mut rect_relative = (viewport_position - target_rect.min) / target_rect.size();
        // Flip the Y co-ordinate origin from the top to the bottom.
        rect_relative.y = 1.0 - rect_relative.y;

        let ndc = rect_relative * 2. - Vec2::ONE;
        let ndc_to_world = camera_transform.to_matrix() * self.computed.clip_from_view.inverse();
        let world_near_plane = ndc_to_world.project_point3(ndc.extend(1.));
        // Using EPSILON because an ndc with Z = 0 returns NaNs.
        let world_far_plane = ndc_to_world.project_point3(ndc.extend(f32::EPSILON));

        // The fallible direction constructor ensures that world_near_plane and world_far_plane aren't NaN.
        Dir3::new(world_far_plane - world_near_plane)
            .map_err(|_| ViewportConversionError::InvalidData)
            .map(|direction| Ray3d {
                origin: world_near_plane,
                direction,
            })
    }

    /// Returns a 2D world position computed from a position on this [`Camera`]'s viewport.
    ///
    /// Useful for 2D cameras and other cameras with an orthographic projection pointing along the Z axis.
    ///
    /// To get the world space coordinates with Normalized Device Coordinates, you should use
    /// [`ndc_to_world`](Self::ndc_to_world).
    ///
    /// # Example
    /// ```no_run
    /// # use bevy_window::Window;
    /// # use bevy_ecs::prelude::*;
    /// # use bevy_transform::prelude::{GlobalTransform, TransformSystems};
    /// # use bevy_camera::Camera;
    /// # use bevy_app::{App, PostUpdate};
    /// #
    /// fn system(camera_query: Single<(&Camera, &GlobalTransform)>, window: Single<&Window>) {
    ///     let (camera, camera_transform) = *camera_query;
    ///
    ///     if let Some(cursor_position) = window.cursor_position()
    ///         // Calculate a world position based on the cursor's position.
    ///         && let Ok(world_pos) = camera.viewport_to_world_2d(camera_transform, cursor_position)
    ///     {
    ///         println!("World position: {world_pos:.2}");
    ///     }
    /// }
    ///
    /// # let mut app = App::new();
    /// // Run the system after transform propagation so the camera's global transform is up-to-date.
    /// app.add_systems(PostUpdate, system.after(TransformSystems::Propagate));
    /// ```
    ///
    /// # Panics
    ///
    /// Will panic if the camera's projection matrix is invalid (has a determinant of 0) and
    /// `glam_assert` is enabled (see [`ndc_to_world`](Self::ndc_to_world).
    pub fn viewport_to_world_2d(
        &self,
        camera_transform: &GlobalTransform,
        viewport_position: Vec2,
    ) -> Result<Vec2, ViewportConversionError> {
        let target_rect = self
            .logical_viewport_rect()
            .ok_or(ViewportConversionError::NoViewportSize)?;
        let mut rect_relative = (viewport_position - target_rect.min) / target_rect.size();

        // Flip the Y co-ordinate origin from the top to the bottom.
        rect_relative.y = 1.0 - rect_relative.y;

        let ndc = rect_relative * 2. - Vec2::ONE;

        let world_near_plane = self
            .ndc_to_world(camera_transform, ndc.extend(1.))
            .ok_or(ViewportConversionError::InvalidData)?;

        Ok(world_near_plane.truncate())
    }

    /// Given a position in world space, use the camera's viewport to compute the Normalized Device Coordinates.
    ///
    /// When the position is within the viewport the values returned will be between -1.0 and 1.0 on the X and Y axes,
    /// and between 0.0 and 1.0 on the Z axis.
    /// To get the coordinates in the render target's viewport dimensions, you should use
    /// [`world_to_viewport`](Self::world_to_viewport).
    ///
    /// Returns `None` if the `camera_transform`, the `world_position`, or the projection matrix defined by
    /// [`Projection`](super::projection::Projection) contain `NAN`.
    ///
    /// # Panics
    ///
    /// Will panic if the `camera_transform` contains `NAN` and the `glam_assert` feature is enabled.
    pub fn world_to_ndc(
        &self,
        camera_transform: &GlobalTransform,
        world_position: Vec3,
    ) -> Option<Vec3> {
        // Build a transformation matrix to convert from world space to NDC using camera data
        let clip_from_world: Mat4 =
            self.computed.clip_from_view * camera_transform.to_matrix().inverse();
        let ndc_space_coords: Vec3 = clip_from_world.project_point3(world_position);

        (!ndc_space_coords.is_nan()).then_some(ndc_space_coords)
    }

    /// Given a position in Normalized Device Coordinates,
    /// use the camera's viewport to compute the world space position.
    ///
    /// When the position is within the viewport the values returned will be between -1.0 and 1.0 on the X and Y axes,
    /// and between 0.0 and 1.0 on the Z axis.
    /// To get the world space coordinates with the viewport position, you should use
    /// [`world_to_viewport`](Self::world_to_viewport).
    ///
    /// Returns `None` if the `camera_transform`, the `world_position`, or the projection matrix defined by
    /// [`Projection`](super::projection::Projection) contain `NAN`.
    ///
    /// # Panics
    ///
    /// Will panic if the projection matrix is invalid (has a determinant of 0) and `glam_assert` is enabled.
    pub fn ndc_to_world(&self, camera_transform: &GlobalTransform, ndc: Vec3) -> Option<Vec3> {
        // Build a transformation matrix to convert from NDC to world space using camera data
        let ndc_to_world = camera_transform.to_matrix() * self.computed.clip_from_view.inverse();

        let world_space_coords = ndc_to_world.project_point3(ndc);

        (!world_space_coords.is_nan()).then_some(world_space_coords)
    }

    /// Converts the depth in Normalized Device Coordinates
    /// to linear view z for perspective projections.
    ///
    /// Note: Depth values in front of the camera will be negative as -z is forward
    pub fn depth_ndc_to_view_z(&self, ndc_depth: f32) -> f32 {
        let near = self.clip_from_view().w_axis.z; // [3][2]
        -near / ndc_depth
    }

    /// Converts the depth in Normalized Device Coordinates
    /// to linear view z for orthographic projections.
    ///
    /// Note: Depth values in front of the camera will be negative as -z is forward
    pub fn depth_ndc_to_view_z_2d(&self, ndc_depth: f32) -> f32 {
        -(self.clip_from_view().w_axis.z - ndc_depth) / self.clip_from_view().z_axis.z
        //                       [3][2]                                         [2][2]
    }
}

/// Control how this [`Camera`] outputs once rendering is completed.
#[derive(Debug, Clone, Copy)]
pub enum CameraOutputMode {
    /// Writes the camera output to configured render target.
    Write {
        /// The blend state that will be used by the pipeline that writes the intermediate render textures to the final render target texture.
        /// If not set, the output will be written as-is, ignoring `clear_color` and the existing data in the final render target texture.
        blend_state: Option<BlendState>,
        /// The clear color operation to perform on the final render target texture.
        clear_color: ClearColorConfig,
    },
    /// Skips writing the camera output to the configured render target. The output will remain in the
    /// Render Target's "intermediate" textures, which a camera with a higher order should write to the render target
    /// using [`CameraOutputMode::Write`]. The "skip" mode can easily prevent render results from being displayed, or cause
    /// them to be lost. Only use this if you know what you are doing!
    /// In camera setups with multiple active cameras rendering to the same [`RenderTarget`], the Skip mode can be used to remove
    /// unnecessary / redundant writes to the final output texture, removing unnecessary render passes.
    Skip,
}

impl Default for CameraOutputMode {
    fn default() -> Self {
        CameraOutputMode::Write {
            blend_state: None,
            clear_color: ClearColorConfig::Default,
        }
    }
}

/// The "target" that a [`Camera`] will render to. For example, this could be a `Window`
/// swapchain or an [`Image`].
#[derive(Debug, Clone, Reflect, From)]
#[reflect(Clone)]
pub enum RenderTarget {
    /// Window to which the camera's view is rendered.
    Window(WindowRef),
    /// Image to which the camera's view is rendered.
    Image(ImageRenderTarget),
    /// Texture View to which the camera's view is rendered.
    /// Useful when the texture view needs to be created outside of Bevy, for example OpenXR.
    TextureView(ManualTextureViewHandle),
}

impl RenderTarget {
    /// Get a handle to the render target's image,
    /// or `None` if the render target is another variant.
    pub fn as_image(&self) -> Option<&Handle<Image>> {
        if let Self::Image(image_target) = self {
            Some(&image_target.handle)
        } else {
            None
        }
    }
}

impl RenderTarget {
    /// Normalize the render target down to a more concrete value, mostly used for equality comparisons.
    pub fn normalize(&self, primary_window: Option<Entity>) -> Option<NormalizedRenderTarget> {
        match self {
            RenderTarget::Window(window_ref) => window_ref
                .normalize(primary_window)
                .map(NormalizedRenderTarget::Window),
            RenderTarget::Image(handle) => Some(NormalizedRenderTarget::Image(handle.clone())),
            RenderTarget::TextureView(id) => Some(NormalizedRenderTarget::TextureView(*id)),
        }
    }
}

/// Normalized version of the render target.
///
/// Once we have this we shouldn't need to resolve it down anymore.
#[derive(Debug, Clone, Reflect, PartialEq, Eq, Hash, PartialOrd, Ord, From)]
#[reflect(Clone, PartialEq, Hash)]
pub enum NormalizedRenderTarget {
    /// Window to which the camera's view is rendered.
    Window(NormalizedWindowRef),
    /// Image to which the camera's view is rendered.
    Image(ImageRenderTarget),
    /// Texture View to which the camera's view is rendered.
    /// Useful when the texture view needs to be created outside of Bevy, for example OpenXR.
    TextureView(ManualTextureViewHandle),
}

/// A unique id that corresponds to a specific `ManualTextureView` in the `ManualTextureViews` collection.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Component, Reflect)]
#[reflect(Component, Default, Debug, PartialEq, Hash, Clone)]
pub struct ManualTextureViewHandle(pub u32);

/// A render target that renders to an [`Image`].
#[derive(Debug, Clone, Reflect, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[reflect(Clone, PartialEq, Hash)]
pub struct ImageRenderTarget {
    /// The image to render to.
    pub handle: Handle<Image>,
    /// The scale factor of the render target image, corresponding to the scale
    /// factor for a window target. This should almost always be 1.0.
    pub scale_factor: FloatOrd,
}

impl From<Handle<Image>> for RenderTarget {
    fn from(handle: Handle<Image>) -> Self {
        Self::Image(handle.into())
    }
}

impl From<Handle<Image>> for ImageRenderTarget {
    fn from(handle: Handle<Image>) -> Self {
        Self {
            handle,
            scale_factor: FloatOrd(1.0),
        }
    }
}

impl Default for RenderTarget {
    fn default() -> Self {
        Self::Window(Default::default())
    }
}

/// This component lets you control the [`TextureUsages`] field of the main texture generated for the camera
#[derive(Component, Clone, Copy, Reflect)]
#[reflect(opaque)]
#[reflect(Component, Default, Clone)]
pub struct CameraMainTextureUsages(pub TextureUsages);

impl Default for CameraMainTextureUsages {
    fn default() -> Self {
        Self(
            TextureUsages::RENDER_ATTACHMENT
                | TextureUsages::TEXTURE_BINDING
                | TextureUsages::COPY_SRC,
        )
    }
}

impl CameraMainTextureUsages {
    pub fn with(mut self, usages: TextureUsages) -> Self {
        self.0 |= usages;
        self
    }
}
