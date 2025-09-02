use crate::{
    batching::gpu_preprocessing::{GpuPreprocessingMode, GpuPreprocessingSupport},
    extract_component::{ExtractComponent, ExtractComponentPlugin},
    extract_resource::{ExtractResource, ExtractResourcePlugin},
    render_asset::RenderAssets,
    render_graph::{CameraDriverNode, InternedRenderSubGraph, RenderGraph, RenderSubGraph},
    render_resource::TextureView,
    sync_world::{RenderEntity, SyncToRenderWorld},
    texture::{GpuImage, ManualTextureViews},
    view::{
        ColorGrading, ExtractedView, ExtractedWindows, Hdr, Msaa, NoIndirectDrawing,
        RenderVisibleEntities, RetainedViewEntity, ViewUniformOffset,
    },
    Extract, ExtractSchedule, Render, RenderApp, RenderSystems,
};

use bevy_app::{App, Plugin, PostStartup, PostUpdate};
use bevy_asset::{AssetEvent, AssetEventSystems, AssetId, Assets};
use bevy_camera::{
    primitives::Frustum,
    visibility::{self, RenderLayers, VisibleEntities},
    Camera, Camera2d, Camera3d, CameraMainTextureUsages, CameraOutputMode, CameraUpdateSystems,
    ClearColor, ClearColorConfig, Exposure, ManualTextureViewHandle, NormalizedRenderTarget,
    Projection, RenderTargetInfo, Viewport,
};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    change_detection::DetectChanges,
    component::Component,
    entity::{ContainsEntity, Entity},
    error::BevyError,
    event::EventReader,
    lifecycle::HookContext,
    prelude::With,
    query::{Has, QueryItem},
    reflect::ReflectComponent,
    resource::Resource,
    schedule::IntoScheduleConfigs,
    system::{Commands, Query, Res, ResMut},
    world::DeferredWorld,
};
use bevy_image::Image;
use bevy_math::{vec2, Mat4, URect, UVec2, UVec4, Vec2};
use bevy_platform::collections::{HashMap, HashSet};
use bevy_reflect::prelude::*;
use bevy_transform::components::GlobalTransform;
use bevy_window::{PrimaryWindow, Window, WindowCreated, WindowResized, WindowScaleFactorChanged};
use tracing::warn;
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
                .add_systems(ExtractSchedule, extract_cameras)
                .add_systems(Render, sort_cameras.in_set(RenderSystems::ManageViews));
            let camera_driver_node = CameraDriverNode::new(render_app.world_mut());
            let mut render_graph = render_app.world_mut().resource_mut::<RenderGraph>();
            render_graph.add_node(crate::graph::CameraDriverLabel, camera_driver_node);
        }
    }
}

fn warn_on_no_render_graph(world: DeferredWorld, HookContext { entity, caller, .. }: HookContext) {
    if !world.entity(entity).contains::<CameraRenderGraph>() {
        warn!("{}Entity {entity} has a `Camera` component, but it doesn't have a render graph configured. Consider adding a `Camera2d` or `Camera3d` component, or manually adding a `CameraRenderGraph` component if you need a custom render graph.", caller.map(|location|format!("{location}: ")).unwrap_or_default());
    }
}

impl ExtractResource for ClearColor {
    type Source = Self;

    fn extract_resource(source: &Self::Source) -> Self {
        source.clone()
    }
}
impl ExtractComponent for CameraMainTextureUsages {
    type QueryData = &'static Self;
    type QueryFilter = ();
    type Out = Self;

    fn extract_component(item: QueryItem<Self::QueryData>) -> Option<Self::Out> {
        Some(*item)
    }
}
impl ExtractComponent for Camera2d {
    type QueryData = &'static Self;
    type QueryFilter = With<Camera>;
    type Out = Self;

    fn extract_component(item: QueryItem<Self::QueryData>) -> Option<Self::Out> {
        Some(item.clone())
    }
}
impl ExtractComponent for Camera3d {
    type QueryData = &'static Self;
    type QueryFilter = With<Camera>;
    type Out = Self;

    fn extract_component(item: QueryItem<Self::QueryData>) -> Option<Self::Out> {
        Some(item.clone())
    }
}

/// Configures the [`RenderGraph`] name assigned to be run for a given [`Camera`] entity.
#[derive(Component, Debug, Deref, DerefMut, Reflect, Clone)]
#[reflect(opaque)]
#[reflect(Component, Debug, Clone)]
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

pub trait NormalizedRenderTargetExt {
    fn get_texture_view<'a>(
        &self,
        windows: &'a ExtractedWindows,
        images: &'a RenderAssets<GpuImage>,
        manual_texture_views: &'a ManualTextureViews,
    ) -> Option<&'a TextureView>;

    /// Retrieves the [`TextureFormat`] of this render target, if it exists.
    fn get_texture_format<'a>(
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
        }
    }

    /// Retrieves the [`TextureFormat`] of this render target, if it exists.
    fn get_texture_format<'a>(
        &self,
        windows: &'a ExtractedWindows,
        images: &'a RenderAssets<GpuImage>,
        manual_texture_views: &'a ManualTextureViews,
    ) -> Option<TextureFormat> {
        match self {
            NormalizedRenderTarget::Window(window_ref) => windows
                .get(&window_ref.entity())
                .and_then(|window| window.swap_chain_texture_format),
            NormalizedRenderTarget::Image(image_target) => images
                .get(&image_target.handle)
                .map(|image| image.texture_format),
            NormalizedRenderTarget::TextureView(id) => {
                manual_texture_views.get(id).map(|tex| tex.format)
            }
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
                    scale_factor: image_target.scale_factor.0,
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
    mut window_resized_events: EventReader<WindowResized>,
    mut window_created_events: EventReader<WindowCreated>,
    mut window_scale_factor_changed_events: EventReader<WindowScaleFactorChanged>,
    mut image_asset_events: EventReader<AssetEvent<Image>>,
    primary_window: Query<Entity, With<PrimaryWindow>>,
    windows: Query<(Entity, &Window)>,
    images: Res<Assets<Image>>,
    manual_texture_views: Res<ManualTextureViews>,
    mut cameras: Query<(&mut Camera, &mut Projection)>,
) -> Result<(), BevyError> {
    let primary_window = primary_window.iter().next();

    let mut changed_window_ids = <HashSet<_>>::default();
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

        if let Some(normalized_target) = &camera.target.normalize(primary_window)
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
            RenderEntity,
            &Camera,
            &CameraRenderGraph,
            &GlobalTransform,
            &VisibleEntities,
            &Frustum,
            Has<Hdr>,
            Option<&ColorGrading>,
            Option<&Exposure>,
            Option<&TemporalJitter>,
            Option<&MipBias>,
            Option<&RenderLayers>,
            Option<&Projection>,
            Has<NoIndirectDrawing>,
        )>,
    >,
    primary_window: Extract<Query<Entity, With<PrimaryWindow>>>,
    gpu_preprocessing_support: Res<GpuPreprocessingSupport>,
    mapper: Extract<Query<&RenderEntity>>,
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
        camera_render_graph,
        transform,
        visible_entities,
        frustum,
        hdr,
        color_grading,
        exposure,
        temporal_jitter,
        mip_bias,
        render_layers,
        projection,
        no_indirect_drawing,
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

            let render_visible_entities = RenderVisibleEntities {
                entities: visible_entities
                    .entities
                    .iter()
                    .map(|(type_id, entities)| {
                        let entities = entities
                            .iter()
                            .map(|entity| {
                                let render_entity = mapper
                                    .get(*entity)
                                    .cloned()
                                    .map(|entity| entity.id())
                                    .unwrap_or(Entity::PLACEHOLDER);
                                (render_entity, (*entity).into())
                            })
                            .collect();
                        (*type_id, entities)
                    })
                    .collect(),
            };

            let mut commands = commands.entity(render_entity);
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

            if let Some(perspective) = projection {
                commands.insert(perspective.clone());
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
/// [`OrthographicProjection`]: bevy_camera::OrthographicProjection
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
#[derive(Component, Reflect, Clone)]
#[reflect(Default, Component)]
pub struct MipBias(pub f32);

impl Default for MipBias {
    fn default() -> Self {
        Self(-1.0)
    }
}
