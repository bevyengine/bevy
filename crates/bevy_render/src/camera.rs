use core::mem;

use crate::{
    batching::gpu_preprocessing::{GpuPreprocessingMode, GpuPreprocessingSupport},
    extract_component::{ExtractComponent, ExtractComponentPlugin},
    extract_resource::{ExtractResource, ExtractResourcePlugin},
    render_asset::RenderAssets,
    render_resource::TextureView,
    sync_component::SyncComponent,
    sync_world::{MainEntity, MainEntityHashSet, RenderEntity, SyncToRenderWorld},
    texture::{GpuImage, ManualTextureViews},
    view::{
        ColorGrading, ExtractedView, ExtractedWindows, Msaa, NoIndirectDrawing,
        RenderVisibleEntities, RenderVisibleMeshEntities, RetainedViewEntity, ViewUniformOffset,
    },
    Extract, ExtractSchedule, Render, RenderApp, RenderSystems,
};

use bevy_app::{App, Plugin, PostStartup, PostUpdate};
use bevy_asset::{AssetEvent, AssetEventSystems, AssetId, Assets};
use bevy_camera::{
    primitives::Frustum,
    visibility::{self, RenderLayers, VisibleEntities},
    Camera, Camera2d, Camera3d, CameraMainTextureUsages, CameraOutputMode, CameraUpdateSystems,
    ClearColor, ClearColorConfig, Exposure, Hdr, ManualTextureViewHandle, MsaaWriteback,
    NormalizedRenderTarget, Projection, RenderTarget, RenderTargetInfo, Viewport,
};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    change_detection::DetectChanges,
    component::Component,
    entity::{ContainsEntity, Entity},
    error::BevyError,
    lifecycle::HookContext,
    message::MessageReader,
    prelude::With,
    query::{Has, QueryItem},
    reflect::ReflectComponent,
    resource::Resource,
    schedule::{InternedScheduleLabel, IntoScheduleConfigs, ScheduleLabel, SystemSet},
    system::{Commands, Query, Res, ResMut},
    world::DeferredWorld,
};
use bevy_image::Image;
use bevy_log::warn;
use bevy_log::warn_once;
use bevy_math::{uvec2, vec2, Mat4, URect, UVec2, UVec4, Vec2};
use bevy_platform::collections::{HashMap, HashSet};
use bevy_reflect::prelude::*;
use bevy_transform::components::GlobalTransform;
use bevy_window::{PrimaryWindow, Window, WindowCreated, WindowResized, WindowScaleFactorChanged};
use itertools::Either;
use wgpu::TextureFormat;

#[derive(Default)]
pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.register_required_components::<Camera, Msaa>()
            .register_required_components::<Camera, SyncToRenderWorld>()
            .register_required_components::<Camera3d, ColorGrading>()
            .register_required_components::<Camera3d, Exposure>()
            .add_plugins((
                ExtractResourcePlugin::<ClearColor>::default(),
                ExtractComponentPlugin::<CameraMainTextureUsages>::default(),
            ))
            .add_systems(PostStartup, camera_system.in_set(CameraUpdateSystems))
            .add_systems(
                PostUpdate,
                camera_system
                    .in_set(CameraUpdateSystems)
                    .before(AssetEventSystems)
                    .before(visibility::update_frusta),
            );
        app.world_mut()
            .register_component_hooks::<Camera>()
            .on_add(warn_on_no_render_graph);

        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .init_resource::<SortedCameras>()
                .init_resource::<DirtySpecializations>()
                .init_resource::<DirtyWireframeSpecializations>()
                .allow_ambiguous_resource::<DirtySpecializations>()
                .allow_ambiguous_resource::<DirtyWireframeSpecializations>()
                .configure_sets(
                    ExtractSchedule,
                    (
                        DirtySpecializationSystems::Clear
                            .before(DirtySpecializationSystems::CheckForChanges),
                        DirtySpecializationSystems::CheckForChanges
                            .before(DirtySpecializationSystems::CheckForRemovals),
                    ),
                )
                .add_systems(
                    ExtractSchedule,
                    (
                        extract_cameras,
                        clear_dirty_specializations.in_set(DirtySpecializationSystems::Clear),
                        clear_dirty_wireframe_specializations
                            .in_set(DirtySpecializationSystems::Clear),
                        expire_specializations_for_views.in_set(RenderSystems::Cleanup),
                        expire_wireframe_specializations_for_views.in_set(RenderSystems::Cleanup),
                    ),
                )
                .add_systems(Render, sort_cameras.in_set(RenderSystems::CreateViews));
        }
    }
}

fn warn_on_no_render_graph(world: DeferredWorld, HookContext { entity, caller, .. }: HookContext) {
    if !world.entity(entity).contains::<CameraRenderGraph>() {
        warn!("{}Entity {entity} has a `Camera` component, but it doesn't have a render graph configured. Usually, adding a `Camera2d` or `Camera3d` component will work.
        However, you may instead need to enable `bevy_core_pipeline`, or may want to manually add a `CameraRenderGraph` component to create a custom render graph.", caller.map(|location|format!("{location}: ")).unwrap_or_default());
    }
}

impl ExtractResource for ClearColor {
    type Source = Self;

    fn extract_resource(source: &Self::Source) -> Self {
        source.clone()
    }
}

impl SyncComponent for CameraMainTextureUsages {
    type Out = Self;
}

impl ExtractComponent for CameraMainTextureUsages {
    type QueryData = &'static Self;
    type QueryFilter = ();

    fn extract_component(item: QueryItem<Self::QueryData>) -> Option<Self::Out> {
        Some(*item)
    }
}

impl SyncComponent for Camera2d {
    type Out = Self;
}

impl ExtractComponent for Camera2d {
    type QueryData = &'static Self;
    type QueryFilter = With<Camera>;

    fn extract_component(item: QueryItem<Self::QueryData>) -> Option<Self::Out> {
        Some(item.clone())
    }
}

impl SyncComponent for Camera3d {
    type Out = Self;
}

impl ExtractComponent for Camera3d {
    type QueryData = &'static Self;
    type QueryFilter = With<Camera>;

    fn extract_component(item: QueryItem<Self::QueryData>) -> Option<Self::Out> {
        Some(item.clone())
    }
}

/// Configures the render schedule to be run for a given [`Camera`] entity.
#[derive(Component, Debug, Deref, DerefMut, Reflect, Clone)]
#[reflect(opaque)]
#[reflect(Component, Debug, Clone)]
pub struct CameraRenderGraph(pub InternedScheduleLabel);

impl CameraRenderGraph {
    /// Creates a new [`CameraRenderGraph`] from a schedule label.
    #[inline]
    pub fn new<T: ScheduleLabel>(schedule: T) -> Self {
        Self(schedule.intern())
    }

    /// Sets the schedule.
    #[inline]
    pub fn set<T: ScheduleLabel>(&mut self, schedule: T) {
        self.0 = schedule.intern();
    }
}

pub trait NormalizedRenderTargetExt {
    fn get_texture_view<'a>(
        &self,
        windows: &'a ExtractedWindows,
        images: &'a RenderAssets<GpuImage>,
        manual_texture_views: &'a ManualTextureViews,
    ) -> Option<&'a TextureView>;

    /// Retrieves the [`TextureFormat`] of this render target, if it exists.
    fn get_texture_view_format<'a>(
        &self,
        windows: &'a ExtractedWindows,
        images: &'a RenderAssets<GpuImage>,
        manual_texture_views: &'a ManualTextureViews,
    ) -> Option<TextureFormat>;

    fn get_render_target_info<'a>(
        &self,
        resolutions: impl IntoIterator<Item = (Entity, &'a Window)>,
        images: &Assets<Image>,
        manual_texture_views: &ManualTextureViews,
    ) -> Result<RenderTargetInfo, MissingRenderTargetInfoError>;

    // Check if this render target is contained in the given changed windows or images.
    fn is_changed(
        &self,
        changed_window_ids: &HashSet<Entity>,
        changed_image_handles: &HashSet<&AssetId<Image>>,
    ) -> bool;
}

impl NormalizedRenderTargetExt for NormalizedRenderTarget {
    fn get_texture_view<'a>(
        &self,
        windows: &'a ExtractedWindows,
        images: &'a RenderAssets<GpuImage>,
        manual_texture_views: &'a ManualTextureViews,
    ) -> Option<&'a TextureView> {
        match self {
            NormalizedRenderTarget::Window(window_ref) => windows
                .get(&window_ref.entity())
                .and_then(|window| window.swap_chain_texture_view.as_ref()),
            NormalizedRenderTarget::Image(image_target) => images
                .get(&image_target.handle)
                .map(|image| &image.texture_view),
            NormalizedRenderTarget::TextureView(id) => {
                manual_texture_views.get(id).map(|tex| &tex.texture_view)
            }
            NormalizedRenderTarget::None { .. } => None,
        }
    }

    /// Retrieves the texture view's [`TextureFormat`] of this render target, if it exists.
    fn get_texture_view_format<'a>(
        &self,
        windows: &'a ExtractedWindows,
        images: &'a RenderAssets<GpuImage>,
        manual_texture_views: &'a ManualTextureViews,
    ) -> Option<TextureFormat> {
        match self {
            NormalizedRenderTarget::Window(window_ref) => windows
                .get(&window_ref.entity())
                .and_then(|window| window.swap_chain_texture_view_format),
            NormalizedRenderTarget::Image(image_target) => {
                images.get(&image_target.handle).map(GpuImage::view_format)
            }
            NormalizedRenderTarget::TextureView(id) => {
                manual_texture_views.get(id).map(|tex| tex.view_format)
            }
            NormalizedRenderTarget::None { .. } => None,
        }
    }

    fn get_render_target_info<'a>(
        &self,
        resolutions: impl IntoIterator<Item = (Entity, &'a Window)>,
        images: &Assets<Image>,
        manual_texture_views: &ManualTextureViews,
    ) -> Result<RenderTargetInfo, MissingRenderTargetInfoError> {
        match self {
            NormalizedRenderTarget::Window(window_ref) => resolutions
                .into_iter()
                .find(|(entity, _)| *entity == window_ref.entity())
                .map(|(_, window)| RenderTargetInfo {
                    physical_size: window.physical_size(),
                    scale_factor: window.resolution.scale_factor(),
                })
                .ok_or(MissingRenderTargetInfoError::Window {
                    window: window_ref.entity(),
                }),
            NormalizedRenderTarget::Image(image_target) => images
                .get(&image_target.handle)
                .map(|image| RenderTargetInfo {
                    physical_size: image.size(),
                    scale_factor: image_target.scale_factor,
                })
                .ok_or(MissingRenderTargetInfoError::Image {
                    image: image_target.handle.id(),
                }),
            NormalizedRenderTarget::TextureView(id) => manual_texture_views
                .get(id)
                .map(|tex| RenderTargetInfo {
                    physical_size: tex.size,
                    scale_factor: 1.0,
                })
                .ok_or(MissingRenderTargetInfoError::TextureView { texture_view: *id }),
            NormalizedRenderTarget::None { width, height } => Ok(RenderTargetInfo {
                physical_size: uvec2(*width, *height),
                scale_factor: 1.0,
            }),
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
            NormalizedRenderTarget::Image(image_target) => {
                changed_image_handles.contains(&image_target.handle.id())
            }
            NormalizedRenderTarget::TextureView(_) => true,
            NormalizedRenderTarget::None { .. } => false,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum MissingRenderTargetInfoError {
    #[error("RenderTarget::Window missing ({window:?}): Make sure the provided entity has a Window component.")]
    Window { window: Entity },
    #[error("RenderTarget::Image missing ({image:?}): Make sure the Image's usages include RenderAssetUsages::MAIN_WORLD.")]
    Image { image: AssetId<Image> },
    #[error("RenderTarget::TextureView missing ({texture_view:?}): make sure the texture view handle was not removed.")]
    TextureView {
        texture_view: ManualTextureViewHandle,
    },
}

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
/// [`OrthographicProjection`]: bevy_camera::OrthographicProjection
/// [`PerspectiveProjection`]: bevy_camera::PerspectiveProjection
pub fn camera_system(
    mut window_resized_reader: MessageReader<WindowResized>,
    mut window_created_reader: MessageReader<WindowCreated>,
    mut window_scale_factor_changed_reader: MessageReader<WindowScaleFactorChanged>,
    mut image_asset_event_reader: MessageReader<AssetEvent<Image>>,
    primary_window: Query<Entity, With<PrimaryWindow>>,
    windows: Query<(Entity, &Window)>,
    images: Res<Assets<Image>>,
    manual_texture_views: Res<ManualTextureViews>,
    mut cameras: Query<(&mut Camera, &RenderTarget, &mut Projection)>,
) -> Result<(), BevyError> {
    let primary_window = primary_window.iter().next();

    let mut changed_window_ids = <HashSet<_>>::default();
    changed_window_ids.extend(window_created_reader.read().map(|event| event.window));
    changed_window_ids.extend(window_resized_reader.read().map(|event| event.window));
    let scale_factor_changed_window_ids: HashSet<_> = window_scale_factor_changed_reader
        .read()
        .map(|event| event.window)
        .collect();
    changed_window_ids.extend(scale_factor_changed_window_ids.clone());

    let changed_image_handles: HashSet<&AssetId<Image>> = image_asset_event_reader
        .read()
        .filter_map(|event| match event {
            AssetEvent::Modified { id } | AssetEvent::Added { id } => Some(id),
            _ => None,
        })
        .collect();

    for (mut camera, render_target, mut camera_projection) in &mut cameras {
        let mut viewport_size = camera
            .viewport
            .as_ref()
            .map(|viewport| viewport.physical_size);

        if let Some(normalized_target) = render_target.normalize(primary_window)
            && (normalized_target.is_changed(&changed_window_ids, &changed_image_handles)
                || camera.is_added()
                || camera_projection.is_changed()
                || camera.computed.old_viewport_size != viewport_size
                || camera.computed.old_sub_camera_view != camera.sub_camera_view)
        {
            let new_computed_target_info = normalized_target.get_render_target_info(
                windows,
                &images,
                &manual_texture_views,
            )?;
            // Check for the scale factor changing, and resize the viewport if needed.
            // This can happen when the window is moved between monitors with different DPIs.
            // Without this, the viewport will take a smaller portion of the window moved to
            // a higher DPI monitor.
            if normalized_target.is_changed(&scale_factor_changed_window_ids, &HashSet::default())
                && let Some(old_scale_factor) = camera
                    .computed
                    .target_info
                    .as_ref()
                    .map(|info| info.scale_factor)
            {
                let resize_factor = new_computed_target_info.scale_factor / old_scale_factor;
                if let Some(ref mut viewport) = camera.viewport {
                    let resize = |vec: UVec2| (vec.as_vec2() * resize_factor).as_uvec2();
                    viewport.physical_position = resize(viewport.physical_position);
                    viewport.physical_size = resize(viewport.physical_size);
                    viewport_size = Some(viewport.physical_size);
                }
            }
            // This check is needed because when changing WindowMode to Fullscreen, the viewport may have invalid
            // arguments due to a sudden change on the window size to a lower value.
            // If the size of the window is lower, the viewport will match that lower value.
            if let Some(viewport) = &mut camera.viewport {
                viewport.clamp_to_size(new_computed_target_info.physical_size);
            }
            camera.computed.target_info = Some(new_computed_target_info);
            if let Some(size) = camera.logical_viewport_size()
                && size.x != 0.0
                && size.y != 0.0
            {
                camera_projection.update(size.x, size.y);
                camera.computed.clip_from_view = match &camera.sub_camera_view {
                    Some(sub_view) => camera_projection.get_clip_from_view_for_sub(sub_view),
                    None => camera_projection.get_clip_from_view(),
                }
            }
        }

        if camera.computed.old_viewport_size != viewport_size {
            camera.computed.old_viewport_size = viewport_size;
        }

        if camera.computed.old_sub_camera_view != camera.sub_camera_view {
            camera.computed.old_sub_camera_view = camera.sub_camera_view;
        }
    }
    Ok(())
}

#[derive(Component, Debug)]
pub struct ExtractedCamera {
    pub target: Option<NormalizedRenderTarget>,
    pub physical_viewport_size: Option<UVec2>,
    pub physical_target_size: Option<UVec2>,
    pub viewport: Option<Viewport>,
    pub schedule: InternedScheduleLabel,
    pub order: isize,
    pub output_mode: CameraOutputMode,
    pub msaa_writeback: MsaaWriteback,
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
            RenderEntity,
            &Camera,
            &RenderTarget,
            &CameraRenderGraph,
            &GlobalTransform,
            &VisibleEntities,
            &Frustum,
            (
                Has<Hdr>,
                Option<&ColorGrading>,
                Option<&Exposure>,
                Option<&TemporalJitter>,
                Option<&MipBias>,
                Option<&RenderLayers>,
                Option<&Projection>,
                Has<NoIndirectDrawing>,
            ),
        )>,
    >,
    primary_window: Extract<Query<Entity, With<PrimaryWindow>>>,
    mut existing_render_visible_entities: Query<&mut RenderVisibleEntities>,
    gpu_preprocessing_support: Res<GpuPreprocessingSupport>,
    mapper: Extract<Query<RenderEntity>>,
) {
    let primary_window = primary_window.iter().next();
    type ExtractedCameraComponents = (
        ExtractedCamera,
        ExtractedView,
        RenderVisibleEntities,
        TemporalJitter,
        MipBias,
        RenderLayers,
        Projection,
        NoIndirectDrawing,
        ViewUniformOffset,
    );
    for (
        main_entity,
        render_entity,
        camera,
        render_target,
        camera_render_graph,
        transform,
        visible_entities,
        frustum,
        (
            hdr,
            color_grading,
            exposure,
            temporal_jitter,
            mip_bias,
            render_layers,
            projection,
            no_indirect_drawing,
        ),
    ) in query.iter()
    {
        if !camera.is_active {
            commands
                .entity(render_entity)
                .remove::<ExtractedCameraComponents>();
            continue;
        }

        let color_grading = color_grading.unwrap_or(&ColorGrading::default()).clone();

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
                commands
                    .entity(render_entity)
                    .remove::<ExtractedCameraComponents>();
                continue;
            }

            let mut render_visible_entities =
                match existing_render_visible_entities.get_mut(render_entity) {
                    Ok(ref mut existing_render_visible_entities) => {
                        mem::take(&mut **existing_render_visible_entities)
                    }
                    Err(_) => RenderVisibleEntities::default(),
                };

            for (visibility_class, visible_mesh_entities) in visible_entities.entities.iter() {
                render_visible_entities
                    .entities
                    .entry(*visibility_class)
                    .or_default()
                    .update_from(&mapper, visible_mesh_entities);
            }

            // Don't delete "unused" visibility classes from
            // `RenderVisibleEntities`. Even if a visibility class seems empty
            // *now*, phases need to be able to find the entities that were just
            // removed from it.

            let mut commands = commands.entity(render_entity);
            commands.insert((
                ExtractedCamera {
                    target: render_target.normalize(primary_window),
                    viewport: camera.viewport.clone(),
                    physical_viewport_size: Some(viewport_size),
                    physical_target_size: Some(target_size),
                    schedule: camera_render_graph.0,
                    order: camera.order,
                    output_mode: camera.output_mode,
                    msaa_writeback: camera.msaa_writeback,
                    clear_color: camera.clear_color,
                    // this will be set in sort_cameras
                    sorted_camera_index_for_target: 0,
                    exposure: exposure
                        .map(Exposure::exposure)
                        .unwrap_or_else(|| Exposure::default().exposure()),
                    hdr,
                },
                ExtractedView {
                    retained_view_entity: RetainedViewEntity::new(main_entity.into(), None, 0),
                    clip_from_view: camera.clip_from_view(),
                    world_from_view: *transform,
                    clip_from_world: None,
                    hdr,
                    viewport: UVec4::new(
                        viewport_origin.x,
                        viewport_origin.y,
                        viewport_size.x,
                        viewport_size.y,
                    ),
                    color_grading,
                    invert_culling: camera.invert_culling,
                },
                render_visible_entities,
                *frustum,
            ));

            if let Some(temporal_jitter) = temporal_jitter {
                commands.insert(temporal_jitter.clone());
            } else {
                commands.remove::<TemporalJitter>();
            }

            if let Some(mip_bias) = mip_bias {
                commands.insert(mip_bias.clone());
            } else {
                commands.remove::<MipBias>();
            }

            if let Some(render_layers) = render_layers {
                commands.insert(render_layers.clone());
            } else {
                commands.remove::<RenderLayers>();
            }

            if let Some(projection) = projection {
                commands.insert(projection.clone());
            } else {
                commands.remove::<Projection>();
            }

            if no_indirect_drawing
                || !matches!(
                    gpu_preprocessing_support.max_supported_mode,
                    GpuPreprocessingMode::Culling
                )
            {
                commands.insert(NoIndirectDrawing);
            } else {
                commands.remove::<NoIndirectDrawing>();
            }
        };
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
        .sort_by(|c1, c2| (c1.order, &c1.target).cmp(&(c2.order, &c2.target)));
    let mut previous_order_target = None;
    let mut ambiguities = <HashSet<_>>::default();
    let mut target_counts = <HashMap<_, _>>::default();
    for sorted_camera in &mut sorted_cameras.0 {
        let new_order_target = (sorted_camera.order, sorted_camera.target.clone());
        if let Some(previous_order_target) = previous_order_target
            && previous_order_target == new_order_target
        {
            ambiguities.insert(new_order_target.clone());
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
        warn_once!(
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
#[derive(Component, Clone, Default, Reflect)]
#[reflect(Default, Component, Clone)]
pub struct TemporalJitter {
    /// Offset is in range [-0.5, 0.5].
    pub offset: Vec2,
}

impl TemporalJitter {
    pub fn jitter_projection(&self, clip_from_view: &mut Mat4, view_size: Vec2) {
        // https://github.com/GPUOpen-LibrariesAndSDKs/FidelityFX-SDK/blob/d7531ae47d8b36a5d4025663e731a47a38be882f/docs/techniques/media/super-resolution-temporal/jitter-space.svg
        let mut jitter = (self.offset * vec2(2.0, -2.0)) / view_size;

        // orthographic
        if clip_from_view.w_axis.w == 1.0 {
            jitter *= vec2(clip_from_view.x_axis.x, clip_from_view.y_axis.y) * 0.5;
        }

        clip_from_view.z_axis.x += jitter.x;
        clip_from_view.z_axis.y += jitter.y;
    }
}

/// Camera component specifying a mip bias to apply when sampling from material textures.
///
/// Often used in conjunction with antialiasing post-process effects to reduce textures blurriness.
#[derive(Component, Reflect, Clone)]
#[reflect(Default, Component)]
pub struct MipBias(pub f32);

impl Default for MipBias {
    fn default() -> Self {
        Self(-1.0)
    }
}

/// Stores information about all entities that have changed in such a way as to
/// potentially require their pipelines to be re-specialized.
///
/// This is conservative; there's no harm, other than performance, in having an
/// entity in this list that doesn't actually need to be re-specialized. Note
/// that the presence of an entity in this list doesn't mean that a new shader
/// will necessarily be compiled; the pipeline cache is checked first.
///
/// This handles 2D meshes, 3D meshes, and sprites. For 2D and 3D wireframes,
/// see [`DirtyWireframeSpecializations`]. The reason for having two separate
/// lists is that a single entity can have both a mesh and a wireframe.
#[derive(Clone, Resource, Default)]
pub struct DirtySpecializations {
    /// All renderable objects that must be re-specialized this frame.
    pub changed_renderables: MainEntityHashSet,

    /// All renderable objects that need their specializations removed this
    /// frame.
    ///
    /// Note that this may include entities in [`Self::changed_renderables`].
    /// This is fine, as old specializations are removed before new ones are
    /// added.
    pub removed_renderables: MainEntityHashSet,

    /// Views that must be respecialized this frame.
    ///
    /// The presence of a view in this list causes all entities that it renders
    /// to be re-specialized.
    pub views: HashSet<RetainedViewEntity>,
}

impl DirtySpecializations {
    /// Returns true if the view has changed in such a way that all specialized
    /// pipelines for entities visible from it must be regenerated.
    pub fn must_wipe_specializations_for_view(&self, view: RetainedViewEntity) -> bool {
        self.views.contains(&view)
    }

    /// Iterates over all entities that need their specializations cleared in
    /// this frame.
    pub fn iter_to_despecialize<'a>(&'a self) -> impl Iterator<Item = &'a MainEntity> {
        // Entities that changed or were removed must be
        // de-specialized.
        self.changed_renderables
            .iter()
            .chain(self.removed_renderables.iter())
    }

    /// Iterates over all entities that need to have their pipelines
    /// re-specialized this frame.
    ///
    /// `last_frame_view_pending_queues` should be the contents of the
    /// [`ViewPendingQueues::prev_frame`] list.
    pub fn iter_to_specialize<'a>(
        &'a self,
        view: RetainedViewEntity,
        render_visible_mesh_entities: &'a RenderVisibleMeshEntities,
        last_frame_view_pending_queues: &'a HashSet<(Entity, MainEntity)>,
    ) -> impl Iterator<Item = &'a (Entity, MainEntity)> {
        (if self.must_wipe_specializations_for_view(view) {
            Either::Left(render_visible_mesh_entities.entities.iter())
        } else {
            Either::Right(render_visible_mesh_entities.added_entities.iter().chain(
                self.changed_renderables.iter().filter_map(|main_entity| {
                    render_visible_mesh_entities
                        .entities
                        .binary_search_by_key(main_entity, |(_, main_entity)| *main_entity)
                        .ok()
                        .map(|index| &render_visible_mesh_entities.entities[index])
                }),
            ))
        })
        .chain(last_frame_view_pending_queues.iter().filter(|entity_pair| {
            render_visible_mesh_entities
                .entities
                .binary_search(entity_pair)
                .is_ok()
        }))
    }

    /// Iterates over all renderables that should be removed from the phase.
    ///
    /// This includes renderables that became invisible this frame, renderables
    /// that are in [`DirtySpecializations::changed_renderables`], and
    /// renderables that are in [`DirtySpecializations::removed_renderables`].
    /// If this view must itself be re-specialized, this will iterate over all
    /// visible entities in addition to those that became invisible.
    pub fn iter_to_dequeue<'a>(
        &'a self,
        view: RetainedViewEntity,
        render_visible_mesh_entities: &'a RenderVisibleMeshEntities,
    ) -> impl Iterator<Item = &'a MainEntity> {
        render_visible_mesh_entities
            .removed_entities
            .iter()
            .map(|(_, main_entity)| main_entity)
            .chain(if self.must_wipe_specializations_for_view(view) {
                // All visible entities must be removed.
                // Note that this includes potentially-invisible entities, but
                // that's OK as they shouldn't be in the caller's bins in the
                // first place.
                Either::Left(
                    render_visible_mesh_entities
                        .entities
                        .iter()
                        .map(|(_, main_entity)| main_entity),
                )
            } else {
                // Only entities that changed must be removed.
                Either::Right(
                    self.changed_renderables
                        .iter()
                        .chain(self.removed_renderables.iter()),
                )
            })
    }

    /// Iterates over all renderables that potentially need to be re-queued.
    ///
    /// This includes both renderables that became visible and those that are in
    /// [`DirtySpecializations::changed_renderables`]. If this view must itself
    /// be re-specialized, this will iterate over all visible renderables.
    ///
    /// `last_frame_view_pending_queues` should be the contents of the
    /// [`ViewPendingQueues::prev_frame`] list.
    pub fn iter_to_queue<'a>(
        &'a self,
        view: RetainedViewEntity,
        render_visible_mesh_entities: &'a RenderVisibleMeshEntities,
        last_frame_view_pending_queues: &'a HashSet<(Entity, MainEntity)>,
    ) -> impl Iterator<Item = &'a (Entity, MainEntity)> {
        (if self.must_wipe_specializations_for_view(view) {
            Either::Left(render_visible_mesh_entities.entities.iter())
        } else {
            Either::Right(render_visible_mesh_entities.added_entities.iter().chain(
                self.changed_renderables.iter().filter_map(|main_entity| {
                    // Only include entities that need respecialization, are
                    // visible, and *didn't* become visible this frame. The
                    // third criterion exists because we already yielded
                    // such entities just prior to this and don't want to
                    // yield the same entity twice.
                    // Note that binary searching works because all lists in
                    // [`RenderVisibleMeshEntities`] are guaranteed to be
                    // sorted.
                    if render_visible_mesh_entities
                        .added_entities
                        .binary_search_by_key(main_entity, |(_, main_entity)| *main_entity)
                        .is_err()
                    {
                        render_visible_mesh_entities
                            .entities
                            .binary_search_by_key(main_entity, |(_, main_entity)| *main_entity)
                            .ok()
                            .map(|index| &render_visible_mesh_entities.entities[index])
                    } else {
                        None
                    }
                }),
            ))
        })
        .chain(last_frame_view_pending_queues.iter().filter(|entity_pair| {
            render_visible_mesh_entities
                .entities
                .binary_search(entity_pair)
                .is_ok()
        }))
    }
}

/// Stores information about all entities that have changed in such a way as to
/// potentially require their wireframe pipelines to be re-specialized.
///
/// This is separate from [`DirtySpecializations`] because a single entity can
/// have both a mesh and a wireframe on it, and the pipelines are treated
/// separately.
///
/// See [`DirtySpecializations`] for more information.
#[derive(Clone, Resource, Default, Deref, DerefMut)]
pub struct DirtyWireframeSpecializations(pub DirtySpecializations);

/// Clears out the [`DirtySpecializations`] resource in preparation for a new
/// frame.
pub fn clear_dirty_specializations(mut dirty_specializations: ResMut<DirtySpecializations>) {
    dirty_specializations.changed_renderables.clear();
    dirty_specializations.removed_renderables.clear();
    dirty_specializations.views.clear();
}

/// Clears out the [`DirtyWireframeSpecializations`] resource in preparation for
/// a new frame.
pub fn clear_dirty_wireframe_specializations(
    mut dirty_wireframe_specializations: ResMut<DirtyWireframeSpecializations>,
) {
    dirty_wireframe_specializations.changed_renderables.clear();
    dirty_wireframe_specializations.removed_renderables.clear();
    dirty_wireframe_specializations.views.clear();
}

/// A system that removes views that don't exist any longer from
/// [`DirtySpecializations`].
pub fn expire_specializations_for_views(
    views: Query<&ExtractedView>,
    mut dirty_specializations: ResMut<DirtySpecializations>,
) {
    let all_live_retained_view_entities: HashSet<_> =
        views.iter().map(|view| view.retained_view_entity).collect();
    dirty_specializations.views.retain(|retained_view_entity| {
        all_live_retained_view_entities.contains(retained_view_entity)
    });
}

/// A system that removes views that don't exist any longer from
/// [`DirtyWireframeSpecializations`].
pub fn expire_wireframe_specializations_for_views(
    views: Query<&ExtractedView>,
    mut dirty_wireframe_specializations: ResMut<DirtyWireframeSpecializations>,
) {
    let all_live_retained_view_entities: HashSet<_> =
        views.iter().map(|view| view.retained_view_entity).collect();
    dirty_wireframe_specializations
        .views
        .retain(|retained_view_entity| {
            all_live_retained_view_entities.contains(retained_view_entity)
        });
}

/// A [`SystemSet`] that contains all systems that mutate the
/// [`DirtySpecializations`] resource and other resources that wrap that type.
///
/// These systems must run in order.
#[derive(SystemSet, Clone, PartialEq, Eq, Debug, Hash)]
pub enum DirtySpecializationSystems {
    /// Systems that clear out [`DirtySpecializations`] types in preparation for
    /// a new frame.
    Clear,

    /// Systems that add entities that need to be re-specialized to
    /// [`DirtySpecializations`].
    CheckForChanges,

    /// Systems that determine which entities need to be removed from render
    /// phases and write the results to [`DirtySpecializations`].
    ///
    /// The set of entities that need to be removed from the render phases can
    /// only be determined after all systems in
    /// [`DirtySpecializationSystems::CheckForChanges`] have run. That's because
    /// these systems check `RemovedComponents` resources, and they have to be
    /// able to distinguish between the case in which an entity was truly made
    /// unrenderable and the case in which an entity appeared in a
    /// `RemovedComponents` table simply because its material *type* changed.
    CheckForRemovals,
}

/// Holds all entities that couldn't be specialized and/or queued because their
/// materials or other dependent resources hadn't loaded yet.
///
/// We might not be able to specialize and/or enqueue a renderable entity if a
/// dependent resource like a material isn't available. In that case, we add the
/// entity to the appropriate list so that we attempt to re-specialize and
/// re-queue it on subsequent frames.
///
/// This type is expected to be placed in a newtype wrapper and stored as a
/// resource: e.g. `PendingMeshMaterialQueues`.
#[derive(Default, Deref, DerefMut)]
pub struct PendingQueues(pub HashMap<RetainedViewEntity, ViewPendingQueues>);

/// Holds all entities that couldn't be specialized and/or queued because their
/// materials and/or other dependent resources hadn't loaded yet for a single
/// view.
///
/// See the documentation of [`PendingQueues`] for more information.
#[derive(Default)]
pub struct ViewPendingQueues {
    /// The entities that couldn't be specialized and/or queued this frame.
    ///
    /// We add to this list during pipeline specialization and queuing.
    pub current_frame: HashSet<(Entity, MainEntity)>,

    /// The entities that we need to re-examine in this frame.
    ///
    /// We attempt to specialize and queue entities in this list every frame, as
    /// long as those entities are still visible.
    pub prev_frame: HashSet<(Entity, MainEntity)>,
}

impl PendingQueues {
    /// Initializes the pending queues for a new frame.
    ///
    /// This method is called during specialization. It creates the queues for
    /// the view if necessary and initializes them.
    pub fn prepare_for_new_frame(
        &mut self,
        retained_view_entity: RetainedViewEntity,
    ) -> &mut ViewPendingQueues {
        let view_pending_queues = self.entry(retained_view_entity).or_default();
        mem::swap(
            &mut view_pending_queues.current_frame,
            &mut view_pending_queues.prev_frame,
        );
        view_pending_queues.current_frame.clear();
        view_pending_queues
    }

    /// Removes any pending queues that belong to views not in the supplied
    /// `all_views` table.
    ///
    /// Specialization systems for phases should call this before returning in
    /// order to clean up resources relating to views that no longer exist.
    pub fn expire_stale_views(&mut self, all_views: &HashSet<RetainedViewEntity>) {
        self.retain(|retained_view_entity, _| all_views.contains(retained_view_entity));
    }
}
