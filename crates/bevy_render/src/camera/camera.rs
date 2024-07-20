use crate::{
    batching::gpu_preprocessing::GpuPreprocessingSupport,
    camera::{CameraProjection, ManualTextureViewHandle, ManualTextureViews},
    prelude::Image,
    primitives::Frustum,
    render_asset::RenderAssets,
    render_graph::{InternedRenderSubGraph, RenderSubGraph},
    render_resource::TextureView,
    texture::GpuImage,
    view::{
        ColorGrading, ExtractedView, ExtractedWindows, GpuCulling, RenderLayers, VisibleEntities,
    },
    Extract,
};
use bevy_asset::{AssetEvent, AssetId, Assets, Handle};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    change_detection::DetectChanges,
    component::Component,
    entity::Entity,
    event::EventReader,
    prelude::With,
    query::Has,
    reflect::ReflectComponent,
    system::{Commands, Query, Res, ResMut, Resource},
};
use bevy_math::{vec2, Dir3, Mat4, Ray3d, Rect, URect, UVec2, UVec4, Vec2, Vec3};
use bevy_reflect::prelude::*;
use bevy_render_macros::ExtractComponent;
use bevy_transform::components::GlobalTransform;
use bevy_utils::{tracing::warn, warn_once};
use bevy_utils::{HashMap, HashSet};
use bevy_window::{
    NormalizedWindowRef, PrimaryWindow, Window, WindowCreated, WindowRef, WindowResized,
    WindowScaleFactorChanged,
};
use std::ops::Range;
use wgpu::{BlendState, TextureFormat, TextureUsages};

use super::{ClearColorConfig, Projection};

/// Render viewport configuration for the [`Camera`] component.
///
/// The viewport defines the area on the render target to which the camera renders its image.
/// You can overlay multiple cameras in a single window using viewports to create effects like
/// split screen, minimaps, and character viewers.
#[derive(Reflect, Debug, Clone)]
#[reflect(Default)]
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
    clip_from_view: Mat4,
    target_info: Option<RenderTargetInfo>,
    // size of the `Viewport`
    old_viewport_size: Option<UVec2>,
}

/// How much energy a `Camera3d` absorbs from incoming light.
///
/// <https://en.wikipedia.org/wiki/Exposure_(photography)>
#[derive(Component, Clone, Copy, Reflect)]
#[reflect_value(Component, Default)]
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
        (-self.ev100).exp2() / 1.2
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
        (self.aperture_f_stops * self.aperture_f_stops * 100.0
            / (self.shutter_speed_s * self.sensitivity_iso))
            .log2()
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

/// The defining [`Component`] for camera entities,
/// storing information about how and what to render through this camera.
///
/// The [`Camera`] component is added to an entity to define the properties of the viewpoint from
/// which rendering occurs. It defines the position of the view to render, the projection method
/// to transform the 3D objects into a 2D image, as well as the render target into which that image
/// is produced.
///
/// Adding a camera is typically done by adding a bundle, either the `Camera2dBundle` or the
/// `Camera3dBundle`.
#[derive(Component, Debug, Reflect, Clone)]
#[reflect(Component, Default)]
pub struct Camera {
    /// If set, this camera will render to the given [`Viewport`] rectangle within the configured [`RenderTarget`].
    pub viewport: Option<Viewport>,
    /// Cameras with a higher order are rendered later, and thus on top of lower order cameras.
    pub order: isize,
    /// If this is set to `true`, this camera will be rendered to its specified [`RenderTarget`]. If `false`, this
    /// camera will not be rendered.
    pub is_active: bool,
    /// Computed values for this camera, such as the projection matrix and the render target size.
    #[reflect(ignore)]
    pub computed: ComputedCameraValues,
    /// The "target" that this camera will render to.
    #[reflect(ignore)]
    pub target: RenderTarget,
    /// If this is set to `true`, the camera will use an intermediate "high dynamic range" render texture.
    /// This allows rendering with a wider range of lighting values.
    pub hdr: bool,
    // todo: reflect this when #6042 lands
    /// The [`CameraOutputMode`] for this camera.
    #[reflect(ignore)]
    pub output_mode: CameraOutputMode,
    /// If this is enabled, a previous camera exists that shares this camera's render target, and this camera has MSAA enabled, then the previous camera's
    /// outputs will be written to the intermediate multi-sampled render target textures for this camera. This enables cameras with MSAA enabled to
    /// "write their results on top" of previous camera results, and include them as a part of their render results. This is enabled by default to ensure
    /// cameras with MSAA enabled layer their results in the same way as cameras without MSAA enabled by default.
    pub msaa_writeback: bool,
    /// The clear color operation to perform on the render target.
    pub clear_color: ClearColorConfig,
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
            hdr: false,
            msaa_writeback: true,
            clear_color: Default::default(),
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
        self.computed.target_info.as_ref().map(|t| t.scale_factor)
    }

    /// The projection matrix computed using this camera's [`CameraProjection`].
    #[inline]
    pub fn clip_from_view(&self) -> Mat4 {
        self.computed.clip_from_view
    }

    /// Given a position in world space, use the camera to compute the viewport-space coordinates.
    ///
    /// To get the coordinates in Normalized Device Coordinates, you should use
    /// [`world_to_ndc`](Self::world_to_ndc).
    ///
    /// Returns `None` if any of these conditions occur:
    /// - The computed coordinates are beyond the near or far plane
    /// - The logical viewport size cannot be computed. See [`logical_viewport_size`](Camera::logical_viewport_size)
    /// - The world coordinates cannot be mapped to the Normalized Device Coordinates. See [`world_to_ndc`](Camera::world_to_ndc)
    /// May also panic if `glam_assert` is enabled. See [`world_to_ndc`](Camera::world_to_ndc).
    #[doc(alias = "world_to_screen")]
    pub fn world_to_viewport(
        &self,
        camera_transform: &GlobalTransform,
        world_position: Vec3,
    ) -> Option<Vec2> {
        let target_size = self.logical_viewport_size()?;
        let ndc_space_coords = self.world_to_ndc(camera_transform, world_position)?;
        // NDC z-values outside of 0 < z < 1 are outside the (implicit) camera frustum and are thus not in viewport-space
        if ndc_space_coords.z < 0.0 || ndc_space_coords.z > 1.0 {
            return None;
        }

        // Once in NDC space, we can discard the z element and rescale x/y to fit the screen
        let mut viewport_position = (ndc_space_coords.truncate() + Vec2::ONE) / 2.0 * target_size;
        // Flip the Y co-ordinate origin from the bottom to the top.
        viewport_position.y = target_size.y - viewport_position.y;
        Some(viewport_position)
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
    /// Returns `None` if any of these conditions occur:
    /// - The logical viewport size cannot be computed. See [`logical_viewport_size`](Camera::logical_viewport_size)
    /// - The near or far plane cannot be computed. This can happen if the `camera_transform`, the `world_position`, or the projection matrix defined by [`CameraProjection`] contain `NAN`.
    /// Panics if the projection matrix is null and `glam_assert` is enabled.
    pub fn viewport_to_world(
        &self,
        camera_transform: &GlobalTransform,
        mut viewport_position: Vec2,
    ) -> Option<Ray3d> {
        let target_size = self.logical_viewport_size()?;
        // Flip the Y co-ordinate origin from the top to the bottom.
        viewport_position.y = target_size.y - viewport_position.y;
        let ndc = viewport_position * 2. / target_size - Vec2::ONE;

        let ndc_to_world =
            camera_transform.compute_matrix() * self.computed.clip_from_view.inverse();
        let world_near_plane = ndc_to_world.project_point3(ndc.extend(1.));
        // Using EPSILON because an ndc with Z = 0 returns NaNs.
        let world_far_plane = ndc_to_world.project_point3(ndc.extend(f32::EPSILON));

        // The fallible direction constructor ensures that world_near_plane and world_far_plane aren't NaN.
        Dir3::new(world_far_plane - world_near_plane).map_or(None, |direction| {
            Some(Ray3d {
                origin: world_near_plane,
                direction,
            })
        })
    }

    /// Returns a 2D world position computed from a position on this [`Camera`]'s viewport.
    ///
    /// Useful for 2D cameras and other cameras with an orthographic projection pointing along the Z axis.
    ///
    /// To get the world space coordinates with Normalized Device Coordinates, you should use
    /// [`ndc_to_world`](Self::ndc_to_world).
    ///
    /// Returns `None` if any of these conditions occur:
    /// - The logical viewport size cannot be computed. See [`logical_viewport_size`](Camera::logical_viewport_size)
    /// - The viewport position cannot be mapped to the world. See [`ndc_to_world`](Camera::ndc_to_world)
    /// May panic. See [`ndc_to_world`](Camera::ndc_to_world).
    pub fn viewport_to_world_2d(
        &self,
        camera_transform: &GlobalTransform,
        mut viewport_position: Vec2,
    ) -> Option<Vec2> {
        let target_size = self.logical_viewport_size()?;
        // Flip the Y co-ordinate origin from the top to the bottom.
        viewport_position.y = target_size.y - viewport_position.y;
        let ndc = viewport_position * 2. / target_size - Vec2::ONE;

        let world_near_plane = self.ndc_to_world(camera_transform, ndc.extend(1.))?;

        Some(world_near_plane.truncate())
    }

    /// Given a position in world space, use the camera's viewport to compute the Normalized Device Coordinates.
    ///
    /// When the position is within the viewport the values returned will be between -1.0 and 1.0 on the X and Y axes,
    /// and between 0.0 and 1.0 on the Z axis.
    /// To get the coordinates in the render target's viewport dimensions, you should use
    /// [`world_to_viewport`](Self::world_to_viewport).
    ///
    /// Returns `None` if the `camera_transform`, the `world_position`, or the projection matrix defined by [`CameraProjection`] contain `NAN`.
    /// Panics if the `camera_transform` contains `NAN` and the `glam_assert` feature is enabled.
    pub fn world_to_ndc(
        &self,
        camera_transform: &GlobalTransform,
        world_position: Vec3,
    ) -> Option<Vec3> {
        // Build a transformation matrix to convert from world space to NDC using camera data
        let clip_from_world: Mat4 =
            self.computed.clip_from_view * camera_transform.compute_matrix().inverse();
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
    /// Returns `None` if the `camera_transform`, the `world_position`, or the projection matrix defined by [`CameraProjection`] contain `NAN`.
    /// Panics if the projection matrix is null and `glam_assert` is enabled.
    pub fn ndc_to_world(&self, camera_transform: &GlobalTransform, ndc: Vec3) -> Option<Vec3> {
        // Build a transformation matrix to convert from NDC to world space using camera data
        let ndc_to_world =
            camera_transform.compute_matrix() * self.computed.clip_from_view.inverse();

        let world_space_coords = ndc_to_world.project_point3(ndc);

        (!world_space_coords.is_nan()).then_some(world_space_coords)
    }
}

/// Control how this camera outputs once rendering is completed.
#[derive(Debug, Clone, Copy)]
pub enum CameraOutputMode {
    /// Writes the camera output to configured render target.
    Write {
        /// The blend state that will be used by the pipeline that writes the intermediate render textures to the final render target texture.
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

/// Configures the [`RenderGraph`](crate::render_graph::RenderGraph) name assigned to be run for a given [`Camera`] entity.
#[derive(Component, Deref, DerefMut, Reflect, Clone)]
#[reflect_value(Component)]
pub struct CameraRenderGraph(InternedRenderSubGraph);

impl CameraRenderGraph {
    /// Creates a new [`CameraRenderGraph`] from any string-like type.
    #[inline]
    pub fn new<T: RenderSubGraph>(name: T) -> Self {
        Self(name.intern())
    }

    /// Sets the graph name.
    #[inline]
    pub fn set<T: RenderSubGraph>(&mut self, name: T) {
        self.0 = name.intern();
    }
}

/// The "target" that a [`Camera`] will render to. For example, this could be a [`Window`]
/// swapchain or an [`Image`].
#[derive(Debug, Clone, Reflect)]
pub enum RenderTarget {
    /// Window to which the camera's view is rendered.
    Window(WindowRef),
    /// Image to which the camera's view is rendered.
    Image(Handle<Image>),
    /// Texture View to which the camera's view is rendered.
    /// Useful when the texture view needs to be created outside of Bevy, for example OpenXR.
    TextureView(ManualTextureViewHandle),
}

impl From<Handle<Image>> for RenderTarget {
    fn from(handle: Handle<Image>) -> Self {
        Self::Image(handle)
    }
}

/// Normalized version of the render target.
///
/// Once we have this we shouldn't need to resolve it down anymore.
#[derive(Debug, Clone, Reflect, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum NormalizedRenderTarget {
    /// Window to which the camera's view is rendered.
    Window(NormalizedWindowRef),
    /// Image to which the camera's view is rendered.
    Image(Handle<Image>),
    /// Texture View to which the camera's view is rendered.
    /// Useful when the texture view needs to be created outside of Bevy, for example OpenXR.
    TextureView(ManualTextureViewHandle),
}

impl Default for RenderTarget {
    fn default() -> Self {
        Self::Window(Default::default())
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

    /// Get a handle to the render target's image,
    /// or `None` if the render target is another variant.
    pub fn as_image(&self) -> Option<&Handle<Image>> {
        if let Self::Image(handle) = self {
            Some(handle)
        } else {
            None
        }
    }
}

impl NormalizedRenderTarget {
    pub fn get_texture_view<'a>(
        &self,
        windows: &'a ExtractedWindows,
        images: &'a RenderAssets<GpuImage>,
        manual_texture_views: &'a ManualTextureViews,
    ) -> Option<&'a TextureView> {
        match self {
            NormalizedRenderTarget::Window(window_ref) => windows
                .get(&window_ref.entity())
                .and_then(|window| window.swap_chain_texture_view.as_ref()),
            NormalizedRenderTarget::Image(image_handle) => {
                images.get(image_handle).map(|image| &image.texture_view)
            }
            NormalizedRenderTarget::TextureView(id) => {
                manual_texture_views.get(id).map(|tex| &tex.texture_view)
            }
        }
    }

    /// Retrieves the [`TextureFormat`] of this render target, if it exists.
    pub fn get_texture_format<'a>(
        &self,
        windows: &'a ExtractedWindows,
        images: &'a RenderAssets<GpuImage>,
        manual_texture_views: &'a ManualTextureViews,
    ) -> Option<TextureFormat> {
        match self {
            NormalizedRenderTarget::Window(window_ref) => windows
                .get(&window_ref.entity())
                .and_then(|window| window.swap_chain_texture_format),
            NormalizedRenderTarget::Image(image_handle) => {
                images.get(image_handle).map(|image| image.texture_format)
            }
            NormalizedRenderTarget::TextureView(id) => {
                manual_texture_views.get(id).map(|tex| tex.format)
            }
        }
    }

    pub fn get_render_target_info<'a>(
        &self,
        resolutions: impl IntoIterator<Item = (Entity, &'a Window)>,
        images: &Assets<Image>,
        manual_texture_views: &ManualTextureViews,
    ) -> Option<RenderTargetInfo> {
        match self {
            NormalizedRenderTarget::Window(window_ref) => resolutions
                .into_iter()
                .find(|(entity, _)| *entity == window_ref.entity())
                .map(|(_, window)| RenderTargetInfo {
                    physical_size: window.physical_size(),
                    scale_factor: window.resolution.scale_factor(),
                }),
            NormalizedRenderTarget::Image(image_handle) => {
                let image = images.get(image_handle)?;
                Some(RenderTargetInfo {
                    physical_size: image.size(),
                    scale_factor: 1.0,
                })
            }
            NormalizedRenderTarget::TextureView(id) => {
                manual_texture_views.get(id).map(|tex| RenderTargetInfo {
                    physical_size: tex.size,
                    scale_factor: 1.0,
                })
            }
        }
    }

    // Check if this render target is contained in the given changed windows or images.
    fn is_changed(
        &self,
        changed_window_ids: &HashSet<Entity>,
        changed_image_handles: &HashSet<&AssetId<Image>>,
    ) -> bool {
        match self {
            NormalizedRenderTarget::Window(window_ref) => {
                changed_window_ids.contains(&window_ref.entity())
            }
            NormalizedRenderTarget::Image(image_handle) => {
                changed_image_handles.contains(&image_handle.id())
            }
            NormalizedRenderTarget::TextureView(_) => true,
        }
    }
}

/// System in charge of updating a [`Camera`] when its window or projection changes.
///
/// The system detects window creation, resize, and scale factor change events to update the camera
/// projection if needed. It also queries any [`CameraProjection`] component associated with the same
/// entity as the [`Camera`] one, to automatically update the camera projection matrix.
///
/// The system function is generic over the camera projection type, and only instances of
/// [`OrthographicProjection`] and [`PerspectiveProjection`] are automatically added to
/// the app, as well as the runtime-selected [`Projection`].
/// The system runs during [`PostUpdate`](bevy_app::PostUpdate).
///
/// ## World Resources
///
/// [`Res<Assets<Image>>`](Assets<Image>) -- For cameras that render to an image, this resource is used to
/// inspect information about the render target. This system will not access any other image assets.
///
/// [`OrthographicProjection`]: crate::camera::OrthographicProjection
/// [`PerspectiveProjection`]: crate::camera::PerspectiveProjection
#[allow(clippy::too_many_arguments)]
pub fn camera_system<T: CameraProjection + Component>(
    mut window_resized_events: EventReader<WindowResized>,
    mut window_created_events: EventReader<WindowCreated>,
    mut window_scale_factor_changed_events: EventReader<WindowScaleFactorChanged>,
    mut image_asset_events: EventReader<AssetEvent<Image>>,
    primary_window: Query<Entity, With<PrimaryWindow>>,
    windows: Query<(Entity, &Window)>,
    images: Res<Assets<Image>>,
    manual_texture_views: Res<ManualTextureViews>,
    mut cameras: Query<(&mut Camera, &mut T)>,
) {
    let primary_window = primary_window.iter().next();

    let mut changed_window_ids = HashSet::new();
    changed_window_ids.extend(window_created_events.read().map(|event| event.window));
    changed_window_ids.extend(window_resized_events.read().map(|event| event.window));
    let scale_factor_changed_window_ids: HashSet<_> = window_scale_factor_changed_events
        .read()
        .map(|event| event.window)
        .collect();
    changed_window_ids.extend(scale_factor_changed_window_ids.clone());

    let changed_image_handles: HashSet<&AssetId<Image>> = image_asset_events
        .read()
        .filter_map(|event| match event {
            AssetEvent::Modified { id } | AssetEvent::Added { id } => Some(id),
            _ => None,
        })
        .collect();

    for (mut camera, mut camera_projection) in &mut cameras {
        let mut viewport_size = camera
            .viewport
            .as_ref()
            .map(|viewport| viewport.physical_size);

        if let Some(normalized_target) = camera.target.normalize(primary_window) {
            if normalized_target.is_changed(&changed_window_ids, &changed_image_handles)
                || camera.is_added()
                || camera_projection.is_changed()
                || camera.computed.old_viewport_size != viewport_size
            {
                let new_computed_target_info = normalized_target.get_render_target_info(
                    &windows,
                    &images,
                    &manual_texture_views,
                );
                // Check for the scale factor changing, and resize the viewport if needed.
                // This can happen when the window is moved between monitors with different DPIs.
                // Without this, the viewport will take a smaller portion of the window moved to
                // a higher DPI monitor.
                if normalized_target.is_changed(&scale_factor_changed_window_ids, &HashSet::new()) {
                    if let (Some(new_scale_factor), Some(old_scale_factor)) = (
                        new_computed_target_info
                            .as_ref()
                            .map(|info| info.scale_factor),
                        camera
                            .computed
                            .target_info
                            .as_ref()
                            .map(|info| info.scale_factor),
                    ) {
                        let resize_factor = new_scale_factor / old_scale_factor;
                        if let Some(ref mut viewport) = camera.viewport {
                            let resize = |vec: UVec2| (vec.as_vec2() * resize_factor).as_uvec2();
                            viewport.physical_position = resize(viewport.physical_position);
                            viewport.physical_size = resize(viewport.physical_size);
                            viewport_size = Some(viewport.physical_size);
                        }
                    }
                }
                // This check is needed because when changing WindowMode to SizedFullscreen, the viewport may have invalid
                // arguments due to a sudden change on the window size to a lower value.
                // If the size of the window is lower, the viewport will match that lower value.
                if let Some(viewport) = &mut camera.viewport {
                    let target_info = &new_computed_target_info;
                    if let Some(target) = target_info {
                        if viewport.physical_size.x > target.physical_size.x {
                            viewport.physical_size.x = target.physical_size.x;
                        }
                        if viewport.physical_size.y > target.physical_size.y {
                            viewport.physical_size.y = target.physical_size.y;
                        }
                    }
                }
                camera.computed.target_info = new_computed_target_info;
                if let Some(size) = camera.logical_viewport_size() {
                    camera_projection.update(size.x, size.y);
                    camera.computed.clip_from_view = camera_projection.get_clip_from_view();
                }
            }
        }

        if camera.computed.old_viewport_size != viewport_size {
            camera.computed.old_viewport_size = viewport_size;
        }
    }
}

/// This component lets you control the [`TextureUsages`] field of the main texture generated for the camera
#[derive(Component, ExtractComponent, Clone, Copy, Reflect)]
#[reflect_value(Component, Default)]
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

#[derive(Component, Debug)]
pub struct ExtractedCamera {
    pub target: Option<NormalizedRenderTarget>,
    pub physical_viewport_size: Option<UVec2>,
    pub physical_target_size: Option<UVec2>,
    pub viewport: Option<Viewport>,
    pub render_graph: InternedRenderSubGraph,
    pub order: isize,
    pub output_mode: CameraOutputMode,
    pub msaa_writeback: bool,
    pub clear_color: ClearColorConfig,
    pub sorted_camera_index_for_target: usize,
    pub exposure: f32,
    pub hdr: bool,
}

pub fn extract_cameras(
    mut commands: Commands,
    query: Extract<
        Query<(
            Entity,
            &Camera,
            &CameraRenderGraph,
            &GlobalTransform,
            &VisibleEntities,
            &Frustum,
            Option<&ColorGrading>,
            Option<&Exposure>,
            Option<&TemporalJitter>,
            Option<&RenderLayers>,
            Option<&Projection>,
            Has<GpuCulling>,
        )>,
    >,
    primary_window: Extract<Query<Entity, With<PrimaryWindow>>>,
    gpu_preprocessing_support: Res<GpuPreprocessingSupport>,
) {
    let primary_window = primary_window.iter().next();
    for (
        entity,
        camera,
        camera_render_graph,
        transform,
        visible_entities,
        frustum,
        color_grading,
        exposure,
        temporal_jitter,
        render_layers,
        projection,
        gpu_culling,
    ) in query.iter()
    {
        let color_grading = color_grading.unwrap_or(&ColorGrading::default()).clone();

        if !camera.is_active {
            continue;
        }

        if let (
            Some(URect {
                min: viewport_origin,
                ..
            }),
            Some(viewport_size),
            Some(target_size),
        ) = (
            camera.physical_viewport_rect(),
            camera.physical_viewport_size(),
            camera.physical_target_size(),
        ) {
            if target_size.x == 0 || target_size.y == 0 {
                continue;
            }

            let mut commands = commands.get_or_spawn(entity);

            commands.insert((
                ExtractedCamera {
                    target: camera.target.normalize(primary_window),
                    viewport: camera.viewport.clone(),
                    physical_viewport_size: Some(viewport_size),
                    physical_target_size: Some(target_size),
                    render_graph: camera_render_graph.0,
                    order: camera.order,
                    output_mode: camera.output_mode,
                    msaa_writeback: camera.msaa_writeback,
                    clear_color: camera.clear_color,
                    // this will be set in sort_cameras
                    sorted_camera_index_for_target: 0,
                    exposure: exposure
                        .map(|e| e.exposure())
                        .unwrap_or_else(|| Exposure::default().exposure()),
                    hdr: camera.hdr,
                },
                ExtractedView {
                    clip_from_view: camera.clip_from_view(),
                    world_from_view: *transform,
                    clip_from_world: None,
                    hdr: camera.hdr,
                    viewport: UVec4::new(
                        viewport_origin.x,
                        viewport_origin.y,
                        viewport_size.x,
                        viewport_size.y,
                    ),
                    color_grading,
                },
                visible_entities.clone(),
                *frustum,
            ));

            if let Some(temporal_jitter) = temporal_jitter {
                commands.insert(temporal_jitter.clone());
            }

            if let Some(render_layers) = render_layers {
                commands.insert(render_layers.clone());
            }

            if let Some(perspective) = projection {
                commands.insert(perspective.clone());
            }

            if gpu_culling {
                if *gpu_preprocessing_support == GpuPreprocessingSupport::Culling {
                    commands.insert(GpuCulling);
                } else {
                    warn_once!(
                        "GPU culling isn't supported on this platform; ignoring `GpuCulling`."
                    );
                }
            }
        }
    }
}

/// Cameras sorted by their order field. This is updated in the [`sort_cameras`] system.
#[derive(Resource, Default)]
pub struct SortedCameras(pub Vec<SortedCamera>);

pub struct SortedCamera {
    pub entity: Entity,
    pub order: isize,
    pub target: Option<NormalizedRenderTarget>,
    pub hdr: bool,
}

pub fn sort_cameras(
    mut sorted_cameras: ResMut<SortedCameras>,
    mut cameras: Query<(Entity, &mut ExtractedCamera)>,
) {
    sorted_cameras.0.clear();
    for (entity, camera) in cameras.iter() {
        sorted_cameras.0.push(SortedCamera {
            entity,
            order: camera.order,
            target: camera.target.clone(),
            hdr: camera.hdr,
        });
    }
    // sort by order and ensure within an order, RenderTargets of the same type are packed together
    sorted_cameras
        .0
        .sort_by(|c1, c2| match c1.order.cmp(&c2.order) {
            std::cmp::Ordering::Equal => c1.target.cmp(&c2.target),
            ord => ord,
        });
    let mut previous_order_target = None;
    let mut ambiguities = HashSet::new();
    let mut target_counts = HashMap::new();
    for sorted_camera in &mut sorted_cameras.0 {
        let new_order_target = (sorted_camera.order, sorted_camera.target.clone());
        if let Some(previous_order_target) = previous_order_target {
            if previous_order_target == new_order_target {
                ambiguities.insert(new_order_target.clone());
            }
        }
        if let Some(target) = &sorted_camera.target {
            let count = target_counts
                .entry((target.clone(), sorted_camera.hdr))
                .or_insert(0usize);
            let (_, mut camera) = cameras.get_mut(sorted_camera.entity).unwrap();
            camera.sorted_camera_index_for_target = *count;
            *count += 1;
        }
        previous_order_target = Some(new_order_target);
    }

    if !ambiguities.is_empty() {
        warn!(
            "Camera order ambiguities detected for active cameras with the following priorities: {:?}. \
            To fix this, ensure there is exactly one Camera entity spawned with a given order for a given RenderTarget. \
            Ambiguities should be resolved because either (1) multiple active cameras were spawned accidentally, which will \
            result in rendering multiple instances of the scene or (2) for cases where multiple active cameras is intentional, \
            ambiguities could result in unpredictable render results.",
            ambiguities
        );
    }
}

/// A subpixel offset to jitter a perspective camera's frustum by.
///
/// Useful for temporal rendering techniques.
///
/// Do not use with [`OrthographicProjection`].
///
/// [`OrthographicProjection`]: crate::camera::OrthographicProjection
#[derive(Component, Clone, Default, Reflect)]
#[reflect(Default, Component)]
pub struct TemporalJitter {
    /// Offset is in range [-0.5, 0.5].
    pub offset: Vec2,
}

impl TemporalJitter {
    pub fn jitter_projection(&self, clip_from_view: &mut Mat4, view_size: Vec2) {
        if clip_from_view.w_axis.w == 1.0 {
            warn!(
                "TemporalJitter not supported with OrthographicProjection. Use PerspectiveProjection instead."
            );
            return;
        }

        // https://github.com/GPUOpen-LibrariesAndSDKs/FidelityFX-SDK/blob/d7531ae47d8b36a5d4025663e731a47a38be882f/docs/techniques/media/super-resolution-temporal/jitter-space.svg
        let jitter = (self.offset * vec2(2.0, -2.0)) / view_size;

        clip_from_view.z_axis.x += jitter.x;
        clip_from_view.z_axis.y += jitter.y;
    }
}

/// Camera component specifying a mip bias to apply when sampling from material textures.
///
/// Often used in conjunction with antialiasing post-process effects to reduce textures blurriness.
#[derive(Default, Component, Reflect)]
#[reflect(Default, Component)]
pub struct MipBias(pub f32);
