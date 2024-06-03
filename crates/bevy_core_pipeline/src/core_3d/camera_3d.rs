use std::{iter, marker::PhantomData};

use crate::{
    core_3d::graph::Core3d,
    tonemapping::{DebandDither, Tonemapping},
};
use arrayvec::ArrayVec;
use bevy_app::{App, Plugin};
use bevy_asset::Assets;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::prelude::*;
use bevy_math::{uvec4, UVec2};
use bevy_reflect::{Reflect, ReflectDeserialize, ReflectSerialize};
use bevy_render::{
    batching::gpu_preprocessing::GpuPreprocessingSupport,
    camera::{
        Camera, CameraMainTextureUsages, CameraRenderGraph, CubemapFaceProjections, Exposure,
        ExtractedCamera, NormalizedRenderTarget, OmnidirectionalProjection, Projection,
        RenderTarget, TemporalJitter, Viewport,
    },
    extract_component::ExtractComponent,
    extract_instances::{ExtractInstance, ExtractedInstances},
    primitives::{CubemapFrusta, Frustum},
    render_resource::{LoadOp, TextureUsages},
    texture::Image,
    view::{
        ColorGrading, CubemapVisibleEntities, ExtractedView, GpuCulling, RenderLayers,
        VisibleEntities,
    },
    Extract, ExtractSchedule, RenderApp,
};
use bevy_transform::prelude::{GlobalTransform, Transform};
use bevy_utils::EntityHashMap;
use bitflags::bitflags;
use nonmax::NonMaxU32;
use serde::{Deserialize, Serialize};

/// Configuration for the "main 3d render graph".
/// The camera coordinate space is right-handed x-right, y-up, z-back.
/// This means "forward" is -Z.
#[derive(Component, Reflect, Clone, ExtractComponent)]
#[extract_component_filter(With<Camera>)]
#[reflect(Component)]
pub struct Camera3d {
    /// The depth clear operation to perform for the main 3d pass.
    pub depth_load_op: Camera3dDepthLoadOp,
    /// The texture usages for the depth texture created for the main 3d pass.
    pub depth_texture_usages: Camera3dDepthTextureUsage,
    /// How many individual steps should be performed in the [`Transmissive3d`](crate::core_3d::Transmissive3d) pass.
    ///
    /// Roughly corresponds to how many “layers of transparency” are rendered for screen space
    /// specular transmissive objects. Each step requires making one additional
    /// texture copy, so it's recommended to keep this number to a resonably low value. Defaults to `1`.
    ///
    /// ### Notes
    ///
    /// - No copies will be performed if there are no transmissive materials currently being rendered,
    ///   regardless of this setting.
    /// - Setting this to `0` disables the screen-space refraction effect entirely, and falls
    ///   back to refracting only the environment map light's texture.
    /// - If set to more than `0`, any opaque [`clear_color`](Camera::clear_color) will obscure the environment
    ///   map light's texture, preventing it from being visible “through” transmissive materials. If you'd like
    ///   to still have the environment map show up in your refractions, you can set the clear color's alpha to `0.0`.
    ///   Keep in mind that depending on the platform and your window settings, this may cause the window to become
    ///   transparent.
    pub screen_space_specular_transmission_steps: usize,
    /// The quality of the screen space specular transmission blur effect, applied to whatever's “behind” transmissive
    /// objects when their `roughness` is greater than `0.0`.
    ///
    /// Higher qualities are more GPU-intensive.
    ///
    /// **Note:** You can get better-looking results at any quality level by enabling TAA. See: [`TemporalAntiAliasPlugin`](crate::experimental::taa::TemporalAntiAliasPlugin).
    pub screen_space_specular_transmission_quality: ScreenSpaceTransmissionQuality,
}

impl Default for Camera3d {
    fn default() -> Self {
        Self {
            depth_load_op: Default::default(),
            depth_texture_usages: TextureUsages::RENDER_ATTACHMENT.into(),
            screen_space_specular_transmission_steps: 1,
            screen_space_specular_transmission_quality: Default::default(),
        }
    }
}

#[derive(Clone, Copy, Reflect, Serialize, Deserialize)]
#[reflect(Serialize, Deserialize)]
pub struct Camera3dDepthTextureUsage(pub u32);

impl From<TextureUsages> for Camera3dDepthTextureUsage {
    fn from(value: TextureUsages) -> Self {
        Self(value.bits())
    }
}
impl From<Camera3dDepthTextureUsage> for TextureUsages {
    fn from(value: Camera3dDepthTextureUsage) -> Self {
        Self::from_bits_truncate(value.0)
    }
}

/// The depth clear operation to perform for the main 3d pass.
#[derive(Reflect, Serialize, Deserialize, Clone, Debug)]
#[reflect(Serialize, Deserialize)]
pub enum Camera3dDepthLoadOp {
    /// Clear with a specified value.
    /// Note that 0.0 is the far plane due to bevy's use of reverse-z projections.
    Clear(f32),
    /// Load from memory.
    Load,
}

impl Default for Camera3dDepthLoadOp {
    fn default() -> Self {
        Camera3dDepthLoadOp::Clear(0.0)
    }
}

impl From<Camera3dDepthLoadOp> for LoadOp<f32> {
    fn from(config: Camera3dDepthLoadOp) -> Self {
        match config {
            Camera3dDepthLoadOp::Clear(x) => LoadOp::Clear(x),
            Camera3dDepthLoadOp::Load => LoadOp::Load,
        }
    }
}

/// The quality of the screen space transmission blur effect, applied to whatever's “behind” transmissive
/// objects when their `roughness` is greater than `0.0`.
///
/// Higher qualities are more GPU-intensive.
///
/// **Note:** You can get better-looking results at any quality level by enabling TAA. See: [`TemporalAntiAliasPlugin`](crate::experimental::taa::TemporalAntiAliasPlugin).
#[derive(Resource, Default, Clone, Copy, Reflect, PartialEq, PartialOrd, Debug)]
#[reflect(Resource)]
pub enum ScreenSpaceTransmissionQuality {
    /// Best performance at the cost of quality. Suitable for lower end GPUs. (e.g. Mobile)
    ///
    /// `num_taps` = 4
    Low,

    /// A balanced option between quality and performance.
    ///
    /// `num_taps` = 8
    #[default]
    Medium,

    /// Better quality. Suitable for high end GPUs. (e.g. Desktop)
    ///
    /// `num_taps` = 16
    High,

    /// The highest quality, suitable for non-realtime rendering. (e.g. Pre-rendered cinematics and photo mode)
    ///
    /// `num_taps` = 32
    Ultra,
}

/// The camera coordinate space is right-handed x-right, y-up, z-back.
/// This means "forward" is -Z.
#[derive(Bundle, Clone)]
pub struct Camera3dBundle {
    pub camera: Camera,
    pub camera_render_graph: CameraRenderGraph,
    pub projection: Projection,
    pub visible_entities: VisibleEntities,
    pub frustum: Frustum,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub camera_3d: Camera3d,
    pub tonemapping: Tonemapping,
    pub deband_dither: DebandDither,
    pub color_grading: ColorGrading,
    pub exposure: Exposure,
    pub main_texture_usages: CameraMainTextureUsages,
}

// NOTE: ideally Perspective and Orthographic defaults can share the same impl, but sadly it breaks rust's type inference
impl Default for Camera3dBundle {
    fn default() -> Self {
        Self {
            camera_render_graph: CameraRenderGraph::new(Core3d),
            camera: Default::default(),
            projection: Default::default(),
            visible_entities: Default::default(),
            frustum: Default::default(),
            transform: Default::default(),
            global_transform: Default::default(),
            camera_3d: Default::default(),
            tonemapping: Default::default(),
            color_grading: Default::default(),
            exposure: Default::default(),
            main_texture_usages: Default::default(),
            deband_dither: DebandDither::Enabled,
        }
    }
}

/// A 360° camera that renders to a cubemap image.
///
/// These cubemap images are typically attached to an environment map light
/// on a light probe, in order to achieve real-time reflective surfaces.
///
/// Internally, these cameras become six subcameras, one for each side of the
/// cube. Consequently, omnidirectional cameras are quite expensive by default.
/// The [`ActiveCubemapSides`] bitfield may be used to reduce this load by
/// rendering to only a subset of the cubemap faces each frame. A common
/// technique is to render to only one cubemap face per frame, cycling through
/// the faces in a round-robin fashion.
#[derive(Bundle)]
pub struct OmnidirectionalCamera3dBundle {
    pub camera: Camera,
    pub camera_render_graph: CameraRenderGraph,
    pub projection: OmnidirectionalProjection,
    pub visible_entities: CubemapVisibleEntities,
    pub active_cubemap_sides: ActiveCubemapSides,
    pub frustum: CubemapFrusta,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub camera_3d: Camera3d,
    pub tonemapping: Tonemapping,
    pub deband_dither: DebandDither,
    pub color_grading: ColorGrading,
    pub exposure: Exposure,
    pub main_texture_usages: CameraMainTextureUsages,
}

impl Default for OmnidirectionalCamera3dBundle {
    fn default() -> Self {
        Self {
            camera: Default::default(),
            camera_render_graph: CameraRenderGraph::new(Core3d),
            projection: Default::default(),
            visible_entities: Default::default(),
            frustum: Default::default(),
            transform: Default::default(),
            global_transform: Default::default(),
            camera_3d: Default::default(),
            tonemapping: Default::default(),
            deband_dither: DebandDither::Enabled,
            color_grading: Default::default(),
            exposure: Default::default(),
            main_texture_usages: Default::default(),
            active_cubemap_sides: Default::default(),
        }
    }
}

#[derive(Resource, Default, Deref, DerefMut)]
pub struct RenderOmnidirectionalCameras(EntityHashMap<Entity, ArrayVec<Entity, 6>>);

bitflags! {
    /// Specifies which sides of an omnidirectional camera will be rendered to
    /// this frame.
    ///
    /// Enabling a flag will cause the renderer to refresh the corresponding
    /// cubemap side on this frame.
    #[derive(Clone, Copy, Component)]
    pub struct ActiveCubemapSides: u8 {
        const X = 0x01;
        const NEG_X = 0x02;
        const Y = 0x04;
        const NEG_Y = 0x08;
        const NEG_Z = 0x10;
        const Z = 0x20;
    }
}

impl Default for ActiveCubemapSides {
    fn default() -> ActiveCubemapSides {
        ActiveCubemapSides::all()
    }
}

/// Extracts components from main world cameras to render world cameras.
///
/// You should generally use this plugin instead of [`ExtractComponentPlugin`]
/// for components on cameras, because in the case of omnidirectional cameras
/// each main world camera will extract to as many as six different sub-cameras,
/// one for each face, and components should be copied onto each face camera.
pub struct ExtractCameraComponentPlugin<C, F = ()> {
    marker: PhantomData<fn() -> (C, F)>,
}

impl<C, F> Plugin for ExtractCameraComponentPlugin<C, F>
where
    C: ExtractComponent,
    C::Out: Clone + 'static,
    F: 'static,
{
    fn build(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app.add_systems(
            ExtractSchedule,
            extract_camera_components::<C>.after(extract_omnidirectional_cameras),
        );
    }
}

impl<C, F> Default for ExtractCameraComponentPlugin<C, F> {
    fn default() -> Self {
        Self {
            marker: Default::default(),
        }
    }
}

/// A plugin that extracts one or more components from a camera into the render
/// world like the [`bevy_render::extract_instances::ExtractInstancesPlugin`]
/// does.
///
/// This plugin should be used instead of
/// [`bevy_render::extract_instances::ExtractInstancesPlugin`] for any component
/// intended to be attached to cameras, because in the case of omnidirectional
/// cameras it'll copy the component to the six individual faces.
pub struct ExtractCameraInstancesPlugin<EI>
where
    EI: ExtractInstance + Clone,
{
    marker: PhantomData<fn() -> EI>,
}

impl<EI> Plugin for ExtractCameraInstancesPlugin<EI>
where
    EI: ExtractInstance + Clone,
{
    fn build(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app.init_resource::<ExtractedInstances<EI>>();
        render_app.add_systems(
            ExtractSchedule,
            extract_instances_from_cameras::<EI>.after(extract_omnidirectional_cameras),
        );
    }
}

impl<EI> ExtractCameraInstancesPlugin<EI>
where
    EI: ExtractInstance + Clone,
{
    /// Creates a new [`ExtractCameraInstancesPlugin`] for a single instance.
    pub fn new() -> ExtractCameraInstancesPlugin<EI> {
        ExtractCameraInstancesPlugin {
            marker: PhantomData,
        }
    }
}

impl<EI> Default for ExtractCameraInstancesPlugin<EI>
where
    EI: ExtractInstance + Clone,
{
    fn default() -> Self {
        Self::new()
    }
}

/// A system that performs the component extraction to the render world for a
/// component with a corresponding [`ExtractCameraComponentPlugin`].
///
/// This system knows about omnidirectional cameras and will copy the component
/// to the individual face cameras as appropriate.
pub fn extract_camera_components<C>(
    mut commands: Commands,
    mut previous_to_spawn_len: Local<usize>,
    omnidirectional_cameras: Res<RenderOmnidirectionalCameras>,
    query: Extract<Query<(Entity, C::QueryData), C::QueryFilter>>,
) where
    C: ExtractComponent,
    C::Out: Clone + 'static,
{
    let mut to_spawn = Vec::with_capacity(*previous_to_spawn_len);

    for (camera, row) in &query {
        let Some(extracted_component) = C::extract_component(row) else {
            continue;
        };

        // If this is an omnidirectional camera, gather up its subcameras;
        // otherwise, just use the camera entity from the query.
        let view_entities: ArrayVec<Entity, 6> = match omnidirectional_cameras.get(&camera) {
            None => iter::once(camera).collect(),
            Some(entities) => entities.clone(),
        };

        for view_entity in view_entities {
            to_spawn.push((view_entity, extracted_component.clone()));
        }
    }

    *previous_to_spawn_len = to_spawn.len();
    commands.insert_or_spawn_batch(to_spawn);
}

/// A system that pulls components from the main world and places them into an
/// [`ExtractedComponents`] resource in the render world, for components present
/// on cameras.
///
/// This system is added by the [`ExtractCameraInstancesPlugin`]. It knows about
/// omnidirectional cameras and will correctly extract components to their six
/// sub-cameras as appropriate.
pub fn extract_instances_from_cameras<EI>(
    mut extracted_instances: ResMut<ExtractedInstances<EI>>,
    omnidirectional_cameras: Res<RenderOmnidirectionalCameras>,
    query: Extract<Query<(Entity, EI::QueryData), EI::QueryFilter>>,
) where
    EI: ExtractInstance + Clone,
{
    extracted_instances.clear();

    for (camera, row) in &query {
        let Some(extract_instance) = EI::extract(row) else {
            continue;
        };

        let view_entities: ArrayVec<Entity, 6> = match omnidirectional_cameras.get(&camera) {
            None => iter::once(camera).collect(),
            Some(entities) => entities.iter().cloned().collect(),
        };
        for view_entity in view_entities {
            extracted_instances.insert(view_entity, extract_instance.clone());
        }
    }
}

/// A system that extracts all omnidirectional cameras to the render world.
///
/// This system populates the [`RenderOmnidirectionalCameras`] resource with
/// newly-created entity IDs for the individual face cameras as it does so. It
/// must run before any systems that add components to the individual face
/// cameras from omnidirectional cameras.
pub fn extract_omnidirectional_cameras(
    mut commands: Commands,
    images: Extract<Res<Assets<Image>>>,
    query: Extract<
        Query<(
            Entity,
            &Camera,
            &Camera3d,
            &Tonemapping,
            &CameraRenderGraph,
            &GlobalTransform,
            &CubemapVisibleEntities,
            &CubemapFrusta,
            (
                Option<&ColorGrading>,
                Option<&Exposure>,
                Option<&TemporalJitter>,
                Option<&RenderLayers>,
            ),
            &OmnidirectionalProjection,
            &CameraMainTextureUsages,
            &ActiveCubemapSides,
            Has<GpuCulling>,
        )>,
    >,
    gpu_preprocessing_support: Res<GpuPreprocessingSupport>,
    mut omnidirectional_cameras: ResMut<RenderOmnidirectionalCameras>,
) {
    omnidirectional_cameras.clear();

    for (
        camera_entity,
        camera,
        camera_3d,
        tonemapping,
        camera_render_graph,
        camera_transform,
        visible_entities,
        cubemap_frusta,
        (color_grading, exposure, temporal_jitter, render_layers),
        projection,
        main_texture_usages,
        active_cubemap_sides,
        gpu_culling,
    ) in query.iter()
    {
        if !camera.is_active {
            continue;
        }

        let RenderTarget::Image(ref cubemap_image_handle) = camera.target else {
            continue;
        };
        let Some(cubemap_image) = images.get(cubemap_image_handle) else {
            continue;
        };

        let view_translation = GlobalTransform::from_translation(camera_transform.translation());
        let color_grading = color_grading.cloned().unwrap_or_default();
        let cubemap_projections = CubemapFaceProjections::new(projection.near);

        // Create the individual subcameras. We may not end up having all six of
        // them if some of them are inactive.
        let mut subcameras: ArrayVec<Entity, 6> = ArrayVec::new();
        for (face_index, (view_rotation, frustum)) in cubemap_projections
            .rotations
            .iter()
            .zip(&cubemap_frusta.frusta)
            .enumerate()
        {
            // If this side is inactive, skip it.
            if !active_cubemap_sides.contains(ActiveCubemapSides::from_bits_retain(1 << face_index))
            {
                continue;
            }

            let mut entity_commands = commands.spawn(ExtractedView {
                clip_from_view: cubemap_projections.projection,
                world_from_view: view_translation * *view_rotation,
                clip_from_world: None,
                hdr: camera.hdr,
                viewport: uvec4(0, 0, cubemap_image.width(), cubemap_image.height()),
                color_grading: color_grading.clone(),
            });

            entity_commands
                .insert(ExtractedCamera {
                    target: Some(NormalizedRenderTarget::Image(cubemap_image_handle.clone())),
                    viewport: Some(Viewport {
                        physical_position: UVec2::ZERO,
                        physical_size: cubemap_image.size(),
                        depth: match camera.viewport {
                            Some(ref viewport) => viewport.depth.clone(),
                            None => 0.0..1.0,
                        },
                    }),
                    physical_viewport_size: Some(cubemap_image.size()),
                    physical_target_size: Some(cubemap_image.size()),
                    render_graph: **camera_render_graph,
                    order: camera.order,
                    output_mode: camera.output_mode,
                    msaa_writeback: camera.msaa_writeback,
                    clear_color: camera.clear_color,
                    sorted_camera_index_for_target: 0,
                    exposure: exposure.cloned().unwrap_or_default().exposure(),
                    render_target_layer: Some(NonMaxU32::try_from(face_index as u32).unwrap()),
                    hdr: camera.hdr,
                })
                .insert(camera_3d.clone())
                .insert(*frustum)
                .insert(*main_texture_usages)
                .insert(*tonemapping)
                .insert(visible_entities.get(face_index).clone());

            if let Some(temporal_jitter) = temporal_jitter {
                entity_commands.insert(temporal_jitter.clone());
            }

            if let Some(render_layers) = render_layers {
                entity_commands.insert(render_layers.clone());
            }

            if gpu_culling {
                gpu_preprocessing_support.maybe_add_gpu_culling(&mut entity_commands);
            }

            subcameras.push(entity_commands.id());
        }

        omnidirectional_cameras.insert(camera_entity, subcameras);
    }
}
