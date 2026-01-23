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
        ColorGrading, ExtractedView, ExtractedWindows, Hdr, NoIndirectDrawing,
        RenderVisibleEntities, RetainedViewEntity, ViewUniformOffset,
    },
    Extract, ExtractSchedule, Render, RenderApp, RenderSystems,
};

use bevy_app::{App, Plugin, PostStartup, PostUpdate};
use bevy_asset::{AssetEvent, AssetEventSystems, AssetId, Assets};
use bevy_camera::{
    color_target::{MainColorTarget, WithMainColorTarget},
    primitives::Frustum,
    visibility::{self, RenderLayers, VisibleEntities},
    Camera, Camera2d, Camera3d, CameraOutputMode, CameraUpdateSystems, ClearColor,
    ClearColorConfig, Exposure, ManualTextureViewHandle, NormalizedRenderTarget, Projection,
    RenderTarget, RenderTargetInfo, Viewport,
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
    query::{Has, QueryEntityError, QueryItem},
    reflect::ReflectComponent,
    resource::Resource,
    schedule::IntoScheduleConfigs,
    system::{Commands, Query, Res, ResMut},
    world::DeferredWorld,
};
use bevy_image::Image;
use bevy_log::warn;
use bevy_math::{uvec2, vec2, Mat4, URect, UVec2, UVec4, Vec2};
use bevy_platform::collections::{HashMap, HashSet};
use bevy_reflect::prelude::*;
use bevy_transform::components::GlobalTransform;
use bevy_window::{PrimaryWindow, Window, WindowCreated, WindowResized, WindowScaleFactorChanged};
use wgpu::TextureFormat;

mod color_target;
pub use color_target::*;

#[derive(Default)]
pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.register_required_components::<Camera, SyncToRenderWorld>()
            .register_required_components::<Camera3d, ColorGrading>()
            .register_required_components::<Camera3d, Exposure>()
            .add_plugins((
                ExtractResourcePlugin::<ClearColor>::default(),
                ExtractComponentPlugin::<MainColorTarget>::default(),
            ))
            .add_systems(
                PostStartup,
                (
                    (
                        configure_camera_color_target,
                        camera_system,
                        configure_camera_color_target,
                    )
                        .chain()
                        .in_set(CameraUpdateSystems),
                    insert_camera_required_components_if_auto_configured
                        .in_set(CameraUpdateSystems),
                ),
            )
            .add_systems(
                PostUpdate,
                (
                    (
                        configure_camera_color_target,
                        camera_system,
                        configure_camera_color_target,
                    )
                        .chain()
                        .in_set(CameraUpdateSystems)
                        .before(AssetEventSystems)
                        .before(visibility::update_frusta),
                    insert_camera_required_components_if_auto_configured
                        .in_set(CameraUpdateSystems),
                ),
            );
        app.world_mut()
            .register_component_hooks::<Camera>()
            .on_add(warn_on_no_render_graph);

        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .init_resource::<SortedCameras>()
                .add_systems(
                    ExtractSchedule,
                    (
                        (sync_camera_color_target_config, extract_cameras).chain(),
                        extract_main_color_target_reads_from,
                    ),
                )
                .add_systems(Render, sort_cameras.in_set(RenderSystems::ManageViews));
            let camera_driver_node = CameraDriverNode::new(render_app.world_mut());
            let mut render_graph = render_app.world_mut().resource_mut::<RenderGraph>();
            render_graph.add_node(crate::graph::CameraDriverLabel, camera_driver_node);
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
        query_main_color_targets: &'a Query<&ExtractedMainColorTarget>,
    ) -> Option<&'a TextureView>;

    /// Retrieves the [`TextureFormat`] of this render target, if it exists.
    fn get_texture_view_format(
        &self,
        windows: &ExtractedWindows,
        images: &RenderAssets<GpuImage>,
        manual_texture_views: &ManualTextureViews,
        query_main_color_targets: &Query<&ExtractedMainColorTarget>,
    ) -> Option<TextureFormat>;

    fn get_render_target_info<'a>(
        &self,
        resolutions: impl IntoIterator<Item = (Entity, &'a Window)>,
        images: &Assets<Image>,
        manual_texture_views: &ManualTextureViews,
        query_main_color_targets: &Query<&MainColorTarget>,
    ) -> Result<RenderTargetInfo, MissingRenderTargetInfoError>;

    // Check if this render target is contained in the given changed windows or images.
    fn is_changed(
        &self,
        changed_window_ids: &HashSet<Entity>,
        changed_image_handles: &HashSet<&AssetId<Image>>,
        query_main_color_targets: &Query<&MainColorTarget>,
    ) -> bool;
}

impl NormalizedRenderTargetExt for NormalizedRenderTarget {
    fn get_texture_view<'a>(
        &self,
        windows: &'a ExtractedWindows,
        images: &'a RenderAssets<GpuImage>,
        manual_texture_views: &'a ManualTextureViews,
        query_main_color_targets: &'a Query<&ExtractedMainColorTarget>,
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
            NormalizedRenderTarget::MainColorTarget { render_entity, .. } => {
                if let Ok(t) = query_main_color_targets.get(render_entity.unwrap())
                    && let Some(image) = images.get(t.main_a)
                {
                    return Some(&image.texture_view);
                }
                None
            }
            NormalizedRenderTarget::None { .. } => None,
        }
    }

    /// Retrieves the texture view's [`TextureFormat`] of this render target, if it exists.
    fn get_texture_view_format(
        &self,
        windows: &ExtractedWindows,
        images: &RenderAssets<GpuImage>,
        manual_texture_views: &ManualTextureViews,
        query_main_color_targets: &Query<&ExtractedMainColorTarget>,
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
            NormalizedRenderTarget::MainColorTarget { render_entity, .. } => {
                if let Ok(t) = query_main_color_targets.get(render_entity.unwrap())
                    && let Some(image) = images.get(t.main_a)
                {
                    return Some(image.view_format());
                }
                None
            }
            NormalizedRenderTarget::None { .. } => None,
        }
    }

    fn get_render_target_info<'a>(
        &self,
        resolutions: impl IntoIterator<Item = (Entity, &'a Window)>,
        images: &Assets<Image>,
        manual_texture_views: &ManualTextureViews,
        query_main_color_targets: &Query<&MainColorTarget>,
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
            NormalizedRenderTarget::MainColorTarget { entity, .. } => {
                match query_main_color_targets.get(*entity) {
                    Ok(t) => {
                        if let Some(image) = images.get(&t.main_a) {
                            Ok(RenderTargetInfo {
                                physical_size: image.size(),
                                scale_factor: 1.0,
                            })
                        } else {
                            Err(MissingRenderTargetInfoError::MainColorTarget {
                                image: Some(t.main_a.id()),
                                query_error: None,
                            })
                        }
                    }
                    Err(err) => Err(MissingRenderTargetInfoError::MainColorTarget {
                        image: None,
                        query_error: Some(err),
                    }),
                }
            }
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
        query_main_color_targets: &Query<&MainColorTarget>,
    ) -> bool {
        match self {
            NormalizedRenderTarget::Window(window_ref) => {
                changed_window_ids.contains(&window_ref.entity())
            }
            NormalizedRenderTarget::Image(image_target) => {
                changed_image_handles.contains(&image_target.handle.id())
            }
            NormalizedRenderTarget::TextureView(_) => true,
            NormalizedRenderTarget::MainColorTarget { entity, .. } => {
                let mut handles = smallvec::SmallVec::<[AssetId<Image>; 3]>::new();
                if let Ok(t) = query_main_color_targets.get(*entity) {
                    handles.push(t.main_a.id());
                    if let Some(b) = &t.main_b {
                        handles.push(b.id());
                    }
                    if let Some(multisampled) = &t.multisampled {
                        handles.push(multisampled.id());
                    }
                }
                handles
                    .iter()
                    .any(|handle| changed_image_handles.contains(handle))
            }

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
    #[error("RenderTarget::MainColorTarget failed to get target info, query error: {query_error:?}, image: {image:?}")]
    MainColorTarget {
        query_error: Option<QueryEntityError>,
        image: Option<AssetId<Image>>,
    },
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
    query_main_color_targets: Query<&MainColorTarget>,
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

        if let Some(normalized_target) = render_target.normalize(primary_window, None)
            && (normalized_target.is_changed(
                &changed_window_ids,
                &changed_image_handles,
                &query_main_color_targets,
            ) || camera.is_added()
                || camera_projection.is_changed()
                || camera.computed.old_viewport_size != viewport_size
                || camera.computed.old_sub_camera_view != camera.sub_camera_view)
        {
            let new_computed_target_info = match normalized_target.get_render_target_info(
                windows,
                &images,
                &manual_texture_views,
                &query_main_color_targets,
            ) {
                Ok(info) => info,
                Err(err) => {
                    // If render target is `MainColorTarget` and query failed, we ignore this error and continue.
                    // Because the entity is not yet spawned by `configure_camera_color_target`,
                    // which runs after and depends on `camera_system` to compute physical target size first.
                    // TODO: Deal with this better.
                    if matches!(
                        err,
                        MissingRenderTargetInfoError::MainColorTarget {
                            query_error: Some(QueryEntityError::QueryDoesNotMatch(..)),
                            image: None
                        }
                    ) {
                        continue;
                    }
                    return Err(err.into());
                }
            };
            // Check for the scale factor changing, and resize the viewport if needed.
            // This can happen when the window is moved between monitors with different DPIs.
            // Without this, the viewport will take a smaller portion of the window moved to
            // a higher DPI monitor.
            if normalized_target.is_changed(
                &scale_factor_changed_window_ids,
                &HashSet::default(),
                &query_main_color_targets,
            ) && let Some(old_scale_factor) = camera
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
    pub output_color_target: Option<NormalizedRenderTarget>,
    pub main_color_target: Entity,
    pub main_color_target_size: UVec2,
    pub viewport: Option<Viewport>,
    pub render_graph: InternedRenderSubGraph,
    pub order: isize,
    pub output_mode: CameraOutputMode,
    pub clear_color: ClearColorConfig,
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
            &WithMainColorTarget,
        )>,
    >,
    query_main_color_targets: Extract<Query<(RenderEntity, &MainColorTarget)>>,
    images: Extract<Res<Assets<Image>>>,
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
        with_main_color_target,
    ) in query.iter()
    {
        if !camera.is_active {
            commands
                .entity(render_entity)
                .remove::<ExtractedCameraComponents>();
            continue;
        }

        let Ok((main_color_target_render_entity, main_color_target)) =
            query_main_color_targets.get(with_main_color_target.0)
        else {
            continue;
        };

        let Some(main_texture_a) = images.get(&main_color_target.main_a) else {
            continue;
        };
        let color_target_format = main_texture_a
            .texture_view_descriptor
            .as_ref()
            .and_then(|v| v.format)
            .unwrap_or(main_texture_a.texture_descriptor.format);
        let main_color_target_size = main_texture_a.size();
        let msaa_samples = if let Some(multisampled) = &main_color_target.multisampled {
            let Some(tex) = images.get(multisampled) else {
                continue;
            };
            tex.texture_descriptor.sample_count
        } else {
            1
        };

        let color_grading = color_grading.unwrap_or(&ColorGrading::default()).clone();

        if let Some(target_size) = camera.physical_target_size() {
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

            let render_target_color_target_render_entity =
                if let RenderTarget::MainColorTarget(entity) = render_target {
                    query_main_color_targets
                        .get(*entity)
                        .ok()
                        .map(|(render_entity, _)| render_entity)
                } else {
                    None
                };
            let output_color_target =
                render_target.normalize(primary_window, render_target_color_target_render_entity);

            let mut commands = commands.entity(render_entity);

            commands.insert((
                ExtractedCamera {
                    output_color_target,
                    main_color_target: main_color_target_render_entity,
                    main_color_target_size,
                    viewport: camera.viewport.clone(),
                    render_graph: camera_render_graph.0,
                    order: camera.order,
                    output_mode: camera.output_mode,
                    clear_color: camera.clear_color,
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
                    viewport: UVec4::new(0, 0, main_color_target_size.x, main_color_target_size.y),
                    color_grading,
                    invert_culling: camera.invert_culling,
                    hdr,
                    color_target_format,
                    msaa_samples,
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
}

pub fn sort_cameras(
    mut sorted_cameras: ResMut<SortedCameras>,
    cameras: Query<(Entity, &ExtractedCamera)>,
) {
    sorted_cameras.0.clear();
    for (entity, camera) in cameras.iter() {
        sorted_cameras.0.push(SortedCamera {
            entity,
            order: camera.order,
        });
    }
    // sort by order and ensure within an order.
    sorted_cameras.0.sort_by(|c1, c2| c1.order.cmp(&c2.order));
    let mut previous_order = None;
    let mut ambiguities = <HashSet<_>>::default();
    for sorted_camera in &mut sorted_cameras.0 {
        let new_order = sorted_camera.order;
        if let Some(previous_order) = previous_order
            && previous_order == new_order
        {
            ambiguities.insert(new_order);
        }
        previous_order = Some(new_order);
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
