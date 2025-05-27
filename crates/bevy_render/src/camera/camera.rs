#![expect(
    clippy::module_inception,
    reason = "The parent module contains all things viewport-related, while this module handles cameras as a component. However, a rename/refactor which should clear up this lint is being discussed; see #17196."
)]
use super::{
    ClearColorConfig, ComputedProjection, Projection, RenderGraphDriver, View, ViewTarget,
};
use crate::{
    batching::gpu_preprocessing::{GpuPreprocessingMode, GpuPreprocessingSupport},
    composition::{manual_texture_view::ManualTextureViews, ViewTarget},
    primitives::{Frustum, SubRect},
    render_phase::Rangefinder3d,
    sync_world::RenderEntity,
    view::{
        ColorGrading, ExtractedView, ExtractedWindows, Hdr, Msaa, NoIndirectDrawing, RenderLayers,
        RenderVisibleEntities, RetainedViewEntity, ViewUniformOffset, Visibility, VisibleEntities,
    },
    Extract,
};
use bevy_asset::{AssetEvent, AssetId, Assets};
use bevy_derive::Deref;
use bevy_ecs::{
    change_detection::DetectChanges,
    component::Component,
    entity::Entity,
    event::EventReader,
    prelude::With,
    query::{Has, QueryData},
    reflect::ReflectComponent,
    relationship::RelationshipSourceCollection,
    system::{lifetimeless::Read, Commands, Query, Res},
};
use bevy_image::Image;
use bevy_math::{ops, vec2, Dir3, Mat4, Ray3d, URect, UVec2, UVec4, Vec2, Vec3};
use bevy_platform::collections::HashSet;
use bevy_reflect::prelude::*;
use bevy_render_macros::ExtractComponent;
use bevy_transform::components::{GlobalTransform, Transform};
use bevy_window::{PrimaryWindow, Window, WindowCreated, WindowResized, WindowScaleFactorChanged};
use thiserror::Error;
use tracing::warn;
use wgpu::{BlendState, TextureUsages};

/// How much energy a `Camera3d` absorbs from incoming light.
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
    /// `world_position`, or the projection matrix defined by [`CameraProjection`] contained `NAN`
    /// (see [`world_to_ndc`][Camera::world_to_ndc] and [`ndc_to_world`][Camera::ndc_to_world]).
    #[error("found NaN while computing NDC")]
    InvalidData,
}

/// The defining [`Component`] for camera entities,
/// storing information about how and what to render through this camera.
///
/// The [`Camera`] component is added to an entity to define the properties of the viewpoint from
/// which rendering occurs. It defines
/// to transform the 3D objects into a 2D image, as well as the render target into which that image
/// is produced.
///
/// Note that a [`Camera`] needs a [`CameraRenderGraph`] to render anything.
/// This is typically provided by adding a [`Camera2d`] or [`Camera3d`] component,
/// but custom render graphs can also be defined. Inserting a [`Camera`] with no render
/// graph will emit an error at runtime.
///
/// [`Camera2d`]: https://docs.rs/bevy/latest/bevy/core_pipeline/core_2d/struct.Camera2d.html
/// [`Camera3d`]: https://docs.rs/bevy/latest/bevy/core_pipeline/core_3d/struct.Camera3d.html
#[derive(Component, Default, Debug, Clone, Reflect)]
#[reflect(Component, Default, Debug, Clone)]
#[require(
    View,
    Frustum,
    CameraMainTextureUsages,
    VisibleEntities,
    Transform,
    Visibility,
    Msaa
)]
pub struct Camera {
    /// If set, this camera will still render to its entire viewport, but its projection will
    /// adjust to only render the specified [`SubRect`] of the total view.
    pub crop: Option<SubRect>,
    /// The blend state that will be used by the pipeline that writes the intermediate render textures to the final view target texture.
    #[reflect(ignore)]
    pub blend_state: Option<BlendState>,
    /// The clear color operation to perform on the final view target texture.
    pub clear_color: ClearColorConfig,
}

impl Camera {
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
        view_target: &ViewTarget,
        camera_transform: &GlobalTransform,
        projection: &ComputedProjection,
        world_position: Vec3,
    ) -> Result<Vec2, ViewportConversionError> {
        let target_rect = view_target.logical_viewport_rect();
        let mut ndc_space_coords = self
            .world_to_ndc(camera_transform, projection, world_position)
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
        view_target: &ViewTarget,
        camera_transform: &GlobalTransform,
        projection: &ComputedProjection,
        world_position: Vec3,
    ) -> Result<Vec3, ViewportConversionError> {
        let target_rect = view_target.logical_viewport_rect();
        let mut ndc_space_coords = self
            .world_to_ndc(camera_transform, projection, world_position)
            .ok_or(ViewportConversionError::InvalidData)?;
        // NDC z-values outside of 0 < z < 1 are outside the (implicit) camera frustum and are thus not in viewport-space
        if ndc_space_coords.z < 0.0 {
            return Err(ViewportConversionError::PastNearPlane);
        }
        if ndc_space_coords.z > 1.0 {
            return Err(ViewportConversionError::PastFarPlane);
        }

        // Stretching ndc depth to value via near plane and negating result to be in positive room again.
        let depth = -self.depth_ndc_to_view_z(projection, ndc_space_coords.z);

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
    /// # Panics
    ///
    /// Will panic if the camera's projection matrix is invalid (has a determinant of 0) and
    /// `glam_assert` is enabled (see [`ndc_to_world`](Self::ndc_to_world).
    pub fn viewport_to_world(
        &self,
        view_target: &ViewTarget,
        camera_transform: &GlobalTransform,
        projection: &ComputedProjection,
        viewport_position: Vec2,
    ) -> Result<Ray3d, ViewportConversionError> {
        let target_rect = view_target.logical_viewport_rect();
        let mut rect_relative = (viewport_position - target_rect.min) / target_rect.size();
        // Flip the Y co-ordinate origin from the top to the bottom.
        rect_relative.y = 1.0 - rect_relative.y;

        let ndc = rect_relative * 2. - Vec2::ONE;
        let ndc_to_world =
            camera_transform.compute_matrix() * projection.clip_from_view().inverse();
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
    /// # Panics
    ///
    /// Will panic if the camera's projection matrix is invalid (has a determinant of 0) and
    /// `glam_assert` is enabled (see [`ndc_to_world`](Self::ndc_to_world).
    pub fn viewport_to_world_2d(
        &self,
        view_target: &ViewTarget,
        camera_transform: &GlobalTransform,
        projection: &ComputedProjection,
        viewport_position: Vec2,
    ) -> Result<Vec2, ViewportConversionError> {
        let target_rect = view_target.logical_viewport_rect();
        let mut rect_relative = (viewport_position - target_rect.min) / target_rect.size();

        // Flip the Y co-ordinate origin from the top to the bottom.
        rect_relative.y = 1.0 - rect_relative.y;

        let ndc = rect_relative * 2. - Vec2::ONE;

        let world_near_plane = self
            .ndc_to_world(camera_transform, projection, ndc.extend(1.))
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
    /// Returns `None` if the `camera_transform`, the `world_position`, or the projection matrix defined by [`CameraProjection`] contain `NAN`.
    ///
    /// # Panics
    ///
    /// Will panic if the `camera_transform` contains `NAN` and the `glam_assert` feature is enabled.
    pub fn world_to_ndc(
        &self,
        camera_transform: &GlobalTransform,
        projection: &ComputedProjection,
        world_position: Vec3,
    ) -> Option<Vec3> {
        // Build a transformation matrix to convert from world space to NDC using camera data
        let clip_from_world: Mat4 =
            projection.clip_from_view() * camera_transform.compute_matrix().inverse();
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
    ///
    /// # Panics
    ///
    /// Will panic if the projection matrix is invalid (has a determinant of 0) and `glam_assert` is enabled.
    pub fn ndc_to_world(
        &self,
        camera_transform: &GlobalTransform,
        projection: &ComputedProjection,
        ndc: Vec3,
    ) -> Option<Vec3> {
        // Build a transformation matrix to convert from NDC to world space using camera data
        let ndc_to_world =
            camera_transform.compute_matrix() * projection.clip_from_view().inverse();

        let world_space_coords = ndc_to_world.project_point3(ndc);

        (!world_space_coords.is_nan()).then_some(world_space_coords)
    }

    /// Converts the depth in Normalized Device Coordinates
    /// to linear view z for perspective projections.
    ///
    /// Note: Depth values in front of the camera will be negative as -z is forward
    pub fn depth_ndc_to_view_z(&self, projection: &ComputedProjection, ndc_depth: f32) -> f32 {
        let near = projection.clip_from_view().w_axis.z; // [3][2]
        -near / ndc_depth
    }

    /// Converts the depth in Normalized Device Coordinates
    /// to linear view z for orthographic projections.
    ///
    /// Note: Depth values in front of the camera will be negative as -z is forward
    pub fn depth_ndc_to_view_z_2d(&self, projection: &ComputedProjection, ndc_depth: f32) -> f32 {
        -(projection.clip_from_view().w_axis.z - ndc_depth) / projection.clip_from_view().z_axis.z
        //                       [3][2]                                         [2][2]
    }
}

// TODO:s for emulating camera_system:
// - detect window changes (scale factor changed, resize, created)
//   - collect changed window ids
// - collect changed image ids
// - detect render target changes, update info
// - clamp viewport to size
// - recalculate projection based on sub-view (KEEP IN CAMERA_SYSTEM)

/// System in charge of updating a [`Camera`] when its window or projection changes.
///
/// The system detects window creation, resize, and scale factor change events to update the camera
/// [`Projection`] if needed.
///
/// ## World Resources
///
/// [`Res<Assets<Image>>`](Assets<Image>) -- For cameras that render to an image, this resource is used to
/// inspect information about the render target. This system will not access any other image assets.
///
/// [`OrthographicProjection`]: crate::camera::OrthographicProjection
/// [`PerspectiveProjection`]: crate::camera::PerspectiveProjection
pub fn camera_system(
    mut window_resized_events: EventReader<WindowResized>,
    mut window_created_events: EventReader<WindowCreated>,
    mut window_scale_factor_changed_events: EventReader<WindowScaleFactorChanged>,
    mut image_asset_events: EventReader<AssetEvent<Image>>,
    primary_window: Query<Entity, With<PrimaryWindow>>,
    windows: Query<(Entity, &Window)>,
    images: Res<Assets<Image>>,
    manual_texture_views: Res<ManualTextureViews>,
    mut cameras: Query<(&mut Camera, &mut Projection)>,
) {
    // let primary_window = primary_window.iter().next();
    //
    // let mut changed_window_ids = <HashSet<_>>::default();
    // changed_window_ids.extend(window_created_events.read().map(|event| event.window));
    // changed_window_ids.extend(window_resized_events.read().map(|event| event.window));
    // let scale_factor_changed_window_ids: HashSet<_> = window_scale_factor_changed_events
    //     .read()
    //     .map(|event| event.window)
    //     .collect();
    // changed_window_ids.extend(scale_factor_changed_window_ids.clone());
    //
    // let changed_image_handles: HashSet<&AssetId<Image>> = image_asset_events
    //     .read()
    //     .filter_map(|event| match event {
    //         AssetEvent::Modified { id } | AssetEvent::Added { id } => Some(id),
    //         _ => None,
    //     })
    //     .collect();
    //
    // for (mut camera, mut camera_projection) in &mut cameras {
    //     let mut viewport_size = camera
    //         .viewport
    //         .as_ref()
    //         .map(|viewport| viewport.physical_size);
    //
    //     if let Some(normalized_target) = camera.target.normalize(primary_window) {
    //         if normalized_target.is_changed(&changed_window_ids, &changed_image_handles)
    //             || camera.is_added()
    //             || camera_projection.is_changed()
    //             || camera.computed.old_viewport_size != viewport_size
    //             || camera.computed.old_crop != camera.crop
    //         {
    //             let new_computed_target_info = normalized_target.get_render_target_info(
    //                 windows,
    //                 &images,
    //                 &manual_texture_views,
    //             );
    //             // Check for the scale factor changing, and resize the viewport if needed.
    //             // This can happen when the window is moved between monitors with different DPIs.
    //             // Without this, the viewport will take a smaller portion of the window moved to
    //             // a higher DPI monitor.
    //             if normalized_target
    //                 .is_changed(&scale_factor_changed_window_ids, &HashSet::default())
    //             {
    //                 if let (Some(new_scale_factor), Some(old_scale_factor)) = (
    //                     new_computed_target_info
    //                         .as_ref()
    //                         .map(|info| info.scale_factor),
    //                     camera
    //                         .computed
    //                         .target_info
    //                         .as_ref()
    //                         .map(|info| info.scale_factor),
    //                 ) {
    //                     let resize_factor = new_scale_factor / old_scale_factor;
    //                     if let Some(ref mut viewport) = camera.viewport {
    //                         let resize = |vec: UVec2| (vec.as_vec2() * resize_factor).as_uvec2();
    //                         viewport.physical_position = resize(viewport.physical_position);
    //                         viewport.physical_size = resize(viewport.physical_size);
    //                         viewport_size = Some(viewport.physical_size);
    //                     }
    //                 }
    //             }
    //             // This check is needed because when changing WindowMode to Fullscreen, the viewport may have invalid
    //             // arguments due to a sudden change on the window size to a lower value.
    //             // If the size of the window is lower, the viewport will match that lower value.
    //             if let Some(viewport) = &mut camera.viewport {
    //                 let target_info = &new_computed_target_info;
    //                 if let Some(target) = target_info {
    //                     viewport.clamp_to_size(target.physical_size);
    //                 }
    //             }
    //             camera.computed.target_info = new_computed_target_info;
    //             if let Some(size) = camera.logical_viewport_size() {
    //                 if size.x != 0.0 && size.y != 0.0 {
    //                     camera_projection.update(size.x, size.y);
    //                     camera.computed.clip_from_view = match &camera.crop {
    //                         Some(sub_view) => {
    //                             camera_projection.get_clip_from_view_for_sub(sub_view)
    //                         }
    //                         None => camera_projection.get_clip_from_view(),
    //                     }
    //                 }
    //             }
    //         }
    //     }
    //
    //     if camera.computed.old_viewport_size != viewport_size {
    //         camera.computed.old_viewport_size = viewport_size;
    //     }
    //
    //     if camera.computed.old_crop != camera.crop {
    //         camera.computed.old_crop = camera.crop;
    //     }
    // }
}

/// This component lets you control the [`TextureUsages`] field of the main texture generated for the camera
#[derive(Component, ExtractComponent, Clone, Copy, Reflect)]
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

#[derive(Component, Debug)]
pub struct ExtractedCamera {
    /// Typically a right-handed projection matrix, one of either:
    ///
    /// Perspective (infinite reverse z)
    /// ```text
    /// f = 1 / tan(fov_y_radians / 2)
    ///
    /// ⎡ f / aspect  0     0   0 ⎤
    /// ⎢          0  f     0   0 ⎥
    /// ⎢          0  0     0  -1 ⎥
    /// ⎣          0  0  near   0 ⎦
    /// ```
    ///
    /// Orthographic
    /// ```text
    /// w = right - left
    /// h = top - bottom
    /// d = near - far
    /// cw = -right - left
    /// ch = -top - bottom
    ///
    /// ⎡  2 / w       0         0  0 ⎤
    /// ⎢      0   2 / h         0  0 ⎥
    /// ⎢      0       0     1 / d  0 ⎥
    /// ⎣ cw / w  ch / h  near / d  1 ⎦
    /// ```
    ///
    /// `clip_from_view[3][3] == 1.0` is the standard way to check if a projection is orthographic
    ///
    /// Custom projections are also possible however.
    pub clip_from_view: Mat4,
    // The view-projection matrix. When provided it is used instead of deriving it from
    // `projection` and `transform` fields, which can be helpful in cases where numerical
    // stability matters and there is a more direct way to derive the view-projection matrix.
    pub clip_from_world: Option<Mat4>,
    pub world_from_view: GlobalTransform,
    pub exposure: f32,
    pub hdr: bool,
    pub msaa_writeback: bool,
    pub color_grading: ColorGrading,
}

impl ExtractedCamera {
    /// Creates a 3D rangefinder for a view
    pub fn rangefinder3d(&self) -> Rangefinder3d {
        Rangefinder3d::from_world_from_view(&self.world_from_view.compute_matrix())
    }
}

pub fn extract_cameras(
    mut commands: Commands,
    query: Extract<
        Query<(
            Entity,
            RenderEntity,
            &View,
            &RenderGraphDriver,
            &Camera,
            &GlobalTransform,
            &VisibleEntities,
            &Frustum,
            Has<Hdr>,
            Option<&ColorGrading>,
            Option<&Exposure>,
            Option<&TemporalJitter>,
            Option<&RenderLayers>,
            Option<&Projection>,
            Has<NoIndirectDrawing>,
        )>,
    >,
    primary_window: Extract<Query<Entity, With<PrimaryWindow>>>,
    gpu_preprocessing_support: Res<GpuPreprocessingSupport>,
    mapper: Extract<Query<&RenderEntity>>,
) {
    // let primary_window = primary_window.iter().next();
    // for (
    //     main_entity,
    //     render_entity,
    //     view,
    //     camera_render_graph,
    //     camera,
    //     transform,
    //     visible_entities,
    //     frustum,
    //     hdr,
    //     color_grading,
    //     exposure,
    //     temporal_jitter,
    //     render_layers,
    //     projection,
    //     no_indirect_drawing,
    // ) in query.iter()
    // {
    //     if !camera.is_active {
    //         commands.entity(render_entity).remove::<(
    //             ExtractedCamera,
    //             ExtractedView,
    //             RenderVisibleEntities,
    //             TemporalJitter,
    //             RenderLayers,
    //             Projection,
    //             NoIndirectDrawing,
    //             ViewUniformOffset,
    //         )>();
    //         continue;
    //     }
    //
    //     let color_grading = color_grading.unwrap_or(&ColorGrading::default()).clone();
    //
    //     if let (
    //         Some(URect {
    //             min: viewport_origin,
    //             ..
    //         }),
    //         Some(viewport_size),
    //         Some(target_size),
    //     ) = (
    //         camera.physical_viewport_rect(),
    //         camera.physical_viewport_size(),
    //         camera.physical_target_size(),
    //     ) {
    //         if target_size.x == 0 || target_size.y == 0 {
    //             continue;
    //         }
    //
    //         let render_visible_entities = RenderVisibleEntities {
    //             entities: visible_entities
    //                 .entities
    //                 .iter()
    //                 .map(|(type_id, entities)| {
    //                     let entities = entities
    //                         .iter()
    //                         .map(|entity| {
    //                             let render_entity = mapper
    //                                 .get(*entity)
    //                                 .cloned()
    //                                 .map(|entity| entity.id())
    //                                 .unwrap_or(Entity::PLACEHOLDER);
    //                             (render_entity, (*entity).into())
    //                         })
    //                         .collect();
    //                     (*type_id, entities)
    //                 })
    //                 .collect(),
    //         };
    //
    //         let mut commands = commands.entity(render_entity);
    //         commands.insert((
    //             ExtractedCamera {
    //                 target: camera.target.normalize(primary_window),
    //                 viewport: camera.viewport.clone(),
    //                 physical_viewport_size: Some(viewport_size),
    //                 physical_target_size: Some(target_size),
    //                 render_graph: camera_render_graph.0,
    //                 order: camera.order,
    //                 output_mode: camera.output_mode,
    //                 msaa_writeback: camera.msaa_writeback,
    //                 clear_color: camera.clear_color,
    //                 // this will be set in sort_cameras
    //                 sorted_camera_index_for_target: 0,
    //                 exposure: exposure
    //                     .map(Exposure::exposure)
    //                     .unwrap_or_else(|| Exposure::default().exposure()),
    //                 hdr,
    //             },
    //             ExtractedView {
    //                 retained_view_entity: RetainedViewEntity::new(main_entity.into(), None, 0),
    //                 clip_from_view: camera.clip_from_view(),
    //                 world_from_view: *transform,
    //                 clip_from_world: None,
    //                 hdr,
    //                 viewport: UVec4::new(
    //                     viewport_origin.x,
    //                     viewport_origin.y,
    //                     viewport_size.x,
    //                     viewport_size.y,
    //                 ),
    //                 color_grading,
    //             },
    //             render_visible_entities,
    //             *frustum,
    //         ));
    //
    //         if let Some(temporal_jitter) = temporal_jitter {
    //             commands.insert(temporal_jitter.clone());
    //         }
    //
    //         if let Some(render_layers) = render_layers {
    //             commands.insert(render_layers.clone());
    //         }
    //
    //         if let Some(perspective) = projection {
    //             commands.insert(perspective.clone());
    //         }
    //
    //         if no_indirect_drawing
    //             || !matches!(
    //                 gpu_preprocessing_support.max_supported_mode,
    //                 GpuPreprocessingMode::Culling
    //             )
    //         {
    //             commands.insert(NoIndirectDrawing);
    //         }
    //     };
    // }
}

/// A subpixel offset to jitter a perspective camera's frustum by.
///
/// Useful for temporal rendering techniques.
///
/// Do not use with [`OrthographicProjection`].
///
/// [`OrthographicProjection`]: crate::camera::OrthographicProjection
#[derive(Component, Clone, Default, Reflect)]
#[reflect(Default, Component, Clone)]
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
