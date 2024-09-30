use super::ExtractedWindows;
use crate::camera::{
    ManualTextureViewHandle, ManualTextureViews, NormalizedRenderTarget, RenderTarget,
};
use crate::render_asset::RenderAssets;
use crate::render_resource::{BindGroupEntries, BufferUsages, TextureUsages, TextureView};
use crate::texture::{GpuImage, OutputColorAttachment};
use crate::view::{
    prepare_view_attachments, prepare_view_targets, ViewTargetAttachments, WindowSurfaces,
};
use crate::{
    prelude::{Image, Shader},
    render_asset::RenderAssetUsages,
    render_resource::{
        binding_types::texture_2d, BindGroup, BindGroupLayout, BindGroupLayoutEntries, Buffer,
        CachedRenderPipelineId, FragmentState, PipelineCache, RenderPipelineDescriptor,
        SpecializedRenderPipeline, SpecializedRenderPipelines, Texture, VertexState,
    },
    renderer::RenderDevice,
    texture::TextureFormatPixelInfo,
    ExtractSchedule, MainWorld, Render, RenderApp, RenderSet,
};
use bevy_app::{First, Plugin, Update};
use bevy_asset::{load_internal_asset, Handle};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::event::event_update_system;
use bevy_ecs::system::SystemState;
use bevy_ecs::{entity::EntityHashMap, prelude::*};
use bevy_hierarchy::DespawnRecursiveExt;
use bevy_reflect::Reflect;
use bevy_tasks::AsyncComputeTaskPool;
use bevy_utils::tracing::{error, info, warn};
use bevy_utils::{default, HashSet};
use bevy_window::{PrimaryWindow, WindowRef};
use std::ops::Deref;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::{borrow::Cow, path::Path};
use wgpu::{
    CommandEncoder, Extent3d, ImageDataLayout, TextureFormat, COPY_BYTES_PER_ROW_ALIGNMENT,
};

#[derive(Event, Deref, DerefMut, Reflect, Debug)]
#[reflect(Debug)]
pub struct ScreenshotCaptured(pub Image);

/// A component that signals to the renderer to capture a screenshot this frame.
///
/// This component should be spawned on a new entity with an observer that will trigger
/// with [`ScreenshotCaptured`] when the screenshot is ready.
///
/// Screenshots are captured asynchronously and may not be available immediately after the frame
/// that the component is spawned on. The observer should be used to handle the screenshot when it
/// is ready.
///
/// Note that the screenshot entity will be despawned after the screenshot is captured and the
/// observer is triggered.
///
/// # Usage
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # use bevy_render::view::screenshot::{save_to_disk, Screenshot};
///
/// fn take_screenshot(mut commands: Commands) {
///    commands.spawn(Screenshot::primary_window())
///       .observe(save_to_disk("screenshot.png"));
/// }
/// ```
#[derive(Component, Deref, DerefMut, Reflect, Debug)]
#[reflect(Component, Debug)]
pub struct Screenshot(pub RenderTarget);

/// A marker component that indicates that a screenshot is currently being captured.
#[derive(Component)]
pub struct Capturing;

/// A marker component that indicates that a screenshot has been captured, the image is ready, and
/// the screenshot entity can be despawned.
#[derive(Component)]
pub struct Captured;

impl Screenshot {
    /// Capture a screenshot of the provided window entity.
    pub fn window(window: Entity) -> Self {
        Self(RenderTarget::Window(WindowRef::Entity(window)))
    }

    /// Capture a screenshot of the primary window, if one exists.
    pub fn primary_window() -> Self {
        Self(RenderTarget::Window(WindowRef::Primary))
    }

    /// Capture a screenshot of the provided render target image.
    pub fn image(image: Handle<Image>) -> Self {
        Self(RenderTarget::Image(image))
    }

    /// Capture a screenshot of the provided manual texture view.
    pub fn texture_view(texture_view: ManualTextureViewHandle) -> Self {
        Self(RenderTarget::TextureView(texture_view))
    }
}

struct ScreenshotPreparedState {
    pub texture: Texture,
    pub buffer: Buffer,
    pub bind_group: BindGroup,
    pub pipeline_id: CachedRenderPipelineId,
    pub size: Extent3d,
}

#[derive(Resource, Deref, DerefMut)]
pub struct CapturedScreenshots(pub Arc<Mutex<Receiver<(Entity, Image)>>>);

#[derive(Resource, Deref, DerefMut, Default)]
struct RenderScreenshotTargets(EntityHashMap<NormalizedRenderTarget>);

#[derive(Resource, Deref, DerefMut, Default)]
struct RenderScreenshotsPrepared(EntityHashMap<ScreenshotPreparedState>);

#[derive(Resource, Deref, DerefMut)]
struct RenderScreenshotsSender(Sender<(Entity, Image)>);

/// Saves the captured screenshot to disk at the provided path.
pub fn save_to_disk(path: impl AsRef<Path>) -> impl FnMut(Trigger<ScreenshotCaptured>) {
    let path = path.as_ref().to_owned();
    move |trigger| {
        let img = trigger.event().deref().clone();
        match img.try_into_dynamic() {
            Ok(dyn_img) => match image::ImageFormat::from_path(&path) {
                Ok(format) => {
                    // discard the alpha channel which stores brightness values when HDR is enabled to make sure
                    // the screenshot looks right
                    let img = dyn_img.to_rgb8();
                    #[cfg(not(target_arch = "wasm32"))]
                    match img.save_with_format(&path, format) {
                        Ok(_) => info!("Screenshot saved to {}", path.display()),
                        Err(e) => error!("Cannot save screenshot, IO error: {e}"),
                    }

                    #[cfg(target_arch = "wasm32")]
                    {
                        let save_screenshot = || {
                            use image::EncodableLayout;
                            use wasm_bindgen::{JsCast, JsValue};

                            let mut image_buffer = std::io::Cursor::new(Vec::new());
                            img.write_to(&mut image_buffer, format)
                                .map_err(|e| JsValue::from_str(&format!("{e}")))?;
                            // SAFETY: `image_buffer` only exist in this closure, and is not used after this line
                            let parts = js_sys::Array::of1(&unsafe {
                                js_sys::Uint8Array::view(image_buffer.into_inner().as_bytes())
                                    .into()
                            });
                            let blob = web_sys::Blob::new_with_u8_array_sequence(&parts)?;
                            let url = web_sys::Url::create_object_url_with_blob(&blob)?;
                            let window = web_sys::window().unwrap();
                            let document = window.document().unwrap();
                            let link = document.create_element("a")?;
                            link.set_attribute("href", &url)?;
                            link.set_attribute(
                                "download",
                                path.file_name()
                                    .and_then(|filename| filename.to_str())
                                    .ok_or_else(|| JsValue::from_str("Invalid filename"))?,
                            )?;
                            let html_element = link.dyn_into::<web_sys::HtmlElement>()?;
                            html_element.click();
                            web_sys::Url::revoke_object_url(&url)?;
                            Ok::<(), JsValue>(())
                        };

                        match (save_screenshot)() {
                            Ok(_) => info!("Screenshot saved to {}", path.display()),
                            Err(e) => error!("Cannot save screenshot, error: {e:?}"),
                        };
                    }
                }
                Err(e) => error!("Cannot save screenshot, requested format not recognized: {e}"),
            },
            Err(e) => error!("Cannot save screenshot, screen format cannot be understood: {e}"),
        }
    }
}

fn clear_screenshots(mut commands: Commands, screenshots: Query<Entity, With<Captured>>) {
    for entity in screenshots.iter() {
        commands.entity(entity).despawn_recursive();
    }
}

pub fn trigger_screenshots(
    mut commands: Commands,
    captured_screenshots: ResMut<CapturedScreenshots>,
) {
    let captured_screenshots = captured_screenshots.lock().unwrap();
    while let Ok((entity, image)) = captured_screenshots.try_recv() {
        commands.entity(entity).insert(Captured);
        commands.trigger_targets(ScreenshotCaptured(image), entity);
    }
}

fn extract_screenshots(
    mut targets: ResMut<RenderScreenshotTargets>,
    mut main_world: ResMut<MainWorld>,
    mut system_state: Local<
        Option<
            SystemState<(
                Commands,
                Query<Entity, With<PrimaryWindow>>,
                Query<(Entity, &Screenshot), Without<Capturing>>,
            )>,
        >,
    >,
    mut seen_targets: Local<HashSet<NormalizedRenderTarget>>,
) {
    if system_state.is_none() {
        *system_state = Some(SystemState::new(&mut main_world));
    }
    let system_state = system_state.as_mut().unwrap();
    let (mut commands, primary_window, screenshots) = system_state.get_mut(&mut main_world);

    targets.clear();
    seen_targets.clear();

    let primary_window = primary_window.iter().next();

    for (entity, screenshot) in screenshots.iter() {
        let render_target = screenshot.0.clone();
        let Some(render_target) = render_target.normalize(primary_window) else {
            warn!(
                "Unknown render target for screenshot, skipping: {:?}",
                render_target
            );
            continue;
        };
        if seen_targets.contains(&render_target) {
            warn!(
                "Duplicate render target for screenshot, skipping entity {:?}: {:?}",
                entity, render_target
            );
            // If we don't despawn the entity here, it will be captured again in the next frame
            commands.entity(entity).despawn_recursive();
            continue;
        }
        seen_targets.insert(render_target.clone());
        targets.insert(entity, render_target);
        commands.entity(entity).insert(Capturing);
    }

    system_state.apply(&mut main_world);
}

#[allow(clippy::too_many_arguments)]
fn prepare_screenshots(
    targets: Res<RenderScreenshotTargets>,
    mut prepared: ResMut<RenderScreenshotsPrepared>,
    window_surfaces: Res<WindowSurfaces>,
    render_device: Res<RenderDevice>,
    screenshot_pipeline: Res<ScreenshotToScreenPipeline>,
    pipeline_cache: Res<PipelineCache>,
    mut pipelines: ResMut<SpecializedRenderPipelines<ScreenshotToScreenPipeline>>,
    images: Res<RenderAssets<GpuImage>>,
    manual_texture_views: Res<ManualTextureViews>,
    mut view_target_attachments: ResMut<ViewTargetAttachments>,
) {
    prepared.clear();
    for (entity, target) in targets.iter() {
        match target {
            NormalizedRenderTarget::Window(window) => {
                let window = window.entity();
                let Some(surface_data) = window_surfaces.surfaces.get(&window) else {
                    warn!("Unknown window for screenshot, skipping: {:?}", window);
                    continue;
                };
                let format = surface_data.configuration.format.add_srgb_suffix();
                let size = Extent3d {
                    width: surface_data.configuration.width,
                    height: surface_data.configuration.height,
                    ..default()
                };
                let (texture_view, state) = prepare_screenshot_state(
                    size,
                    format,
                    &render_device,
                    &screenshot_pipeline,
                    &pipeline_cache,
                    &mut pipelines,
                );
                prepared.insert(*entity, state);
                view_target_attachments.insert(
                    target.clone(),
                    OutputColorAttachment::new(texture_view.clone(), format.add_srgb_suffix()),
                );
            }
            NormalizedRenderTarget::Image(image) => {
                let Some(gpu_image) = images.get(image) else {
                    warn!("Unknown image for screenshot, skipping: {:?}", image);
                    continue;
                };
                let format = gpu_image.texture_format;
                let size = Extent3d {
                    width: gpu_image.size.x,
                    height: gpu_image.size.y,
                    ..default()
                };
                let (texture_view, state) = prepare_screenshot_state(
                    size,
                    format,
                    &render_device,
                    &screenshot_pipeline,
                    &pipeline_cache,
                    &mut pipelines,
                );
                prepared.insert(*entity, state);
                view_target_attachments.insert(
                    target.clone(),
                    OutputColorAttachment::new(texture_view.clone(), format.add_srgb_suffix()),
                );
            }
            NormalizedRenderTarget::TextureView(texture_view) => {
                let Some(manual_texture_view) = manual_texture_views.get(texture_view) else {
                    warn!(
                        "Unknown manual texture view for screenshot, skipping: {:?}",
                        texture_view
                    );
                    continue;
                };
                let format = manual_texture_view.format;
                let size = Extent3d {
                    width: manual_texture_view.size.x,
                    height: manual_texture_view.size.y,
                    ..default()
                };
                let (texture_view, state) = prepare_screenshot_state(
                    size,
                    format,
                    &render_device,
                    &screenshot_pipeline,
                    &pipeline_cache,
                    &mut pipelines,
                );
                prepared.insert(*entity, state);
                view_target_attachments.insert(
                    target.clone(),
                    OutputColorAttachment::new(texture_view.clone(), format.add_srgb_suffix()),
                );
            }
        }
    }
}

fn prepare_screenshot_state(
    size: Extent3d,
    format: TextureFormat,
    render_device: &RenderDevice,
    pipeline: &ScreenshotToScreenPipeline,
    pipeline_cache: &PipelineCache,
    pipelines: &mut SpecializedRenderPipelines<ScreenshotToScreenPipeline>,
) -> (TextureView, ScreenshotPreparedState) {
    let texture = render_device.create_texture(&wgpu::TextureDescriptor {
        label: Some("screenshot-capture-rendertarget"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: TextureUsages::RENDER_ATTACHMENT
            | TextureUsages::COPY_SRC
            | TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let texture_view = texture.create_view(&Default::default());
    let buffer = render_device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("screenshot-transfer-buffer"),
        size: get_aligned_size(size.width, size.height, format.pixel_size() as u32) as u64,
        usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let bind_group = render_device.create_bind_group(
        "screenshot-to-screen-bind-group",
        &pipeline.bind_group_layout,
        &BindGroupEntries::single(&texture_view),
    );
    let pipeline_id = pipelines.specialize(pipeline_cache, pipeline, format);

    (
        texture_view,
        ScreenshotPreparedState {
            texture,
            buffer,
            bind_group,
            pipeline_id,
            size,
        },
    )
}

pub struct ScreenshotPlugin;

const SCREENSHOT_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(11918575842344596158);

impl Plugin for ScreenshotPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_systems(
            First,
            clear_screenshots
                .after(event_update_system)
                .before(apply_deferred),
        )
        .add_systems(Update, trigger_screenshots)
        .register_type::<Screenshot>()
        .register_type::<ScreenshotCaptured>();

        load_internal_asset!(
            app,
            SCREENSHOT_SHADER_HANDLE,
            "screenshot.wgsl",
            Shader::from_wgsl
        );
    }

    fn finish(&self, app: &mut bevy_app::App) {
        let (tx, rx) = std::sync::mpsc::channel();
        app.insert_resource(CapturedScreenshots(Arc::new(Mutex::new(rx))));

        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .insert_resource(RenderScreenshotsSender(tx))
                .init_resource::<RenderScreenshotTargets>()
                .init_resource::<RenderScreenshotsPrepared>()
                .init_resource::<SpecializedRenderPipelines<ScreenshotToScreenPipeline>>()
                .add_systems(ExtractSchedule, extract_screenshots.ambiguous_with_all())
                .add_systems(
                    Render,
                    prepare_screenshots
                        .after(prepare_view_attachments)
                        .before(prepare_view_targets)
                        .in_set(RenderSet::ManageViews),
                );
        }
    }
}

pub(crate) fn align_byte_size(value: u32) -> u32 {
    value + (COPY_BYTES_PER_ROW_ALIGNMENT - (value % COPY_BYTES_PER_ROW_ALIGNMENT))
}

pub(crate) fn get_aligned_size(width: u32, height: u32, pixel_size: u32) -> u32 {
    height * align_byte_size(width * pixel_size)
}

pub(crate) fn layout_data(width: u32, height: u32, format: TextureFormat) -> ImageDataLayout {
    ImageDataLayout {
        bytes_per_row: if height > 1 {
            // 1 = 1 row
            Some(get_aligned_size(width, 1, format.pixel_size() as u32))
        } else {
            None
        },
        rows_per_image: None,
        ..Default::default()
    }
}

#[derive(Resource)]
pub struct ScreenshotToScreenPipeline {
    pub bind_group_layout: BindGroupLayout,
}

impl FromWorld for ScreenshotToScreenPipeline {
    fn from_world(render_world: &mut World) -> Self {
        let device = render_world.resource::<RenderDevice>();

        let bind_group_layout = device.create_bind_group_layout(
            "screenshot-to-screen-bgl",
            &BindGroupLayoutEntries::single(
                wgpu::ShaderStages::FRAGMENT,
                texture_2d(wgpu::TextureSampleType::Float { filterable: false }),
            ),
        );

        Self { bind_group_layout }
    }
}

impl SpecializedRenderPipeline for ScreenshotToScreenPipeline {
    type Key = TextureFormat;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        RenderPipelineDescriptor {
            label: Some(Cow::Borrowed("screenshot-to-screen")),
            layout: vec![self.bind_group_layout.clone()],
            vertex: VertexState {
                buffers: vec![],
                shader_defs: vec![],
                entry_point: Cow::Borrowed("vs_main"),
                shader: SCREENSHOT_SHADER_HANDLE,
            },
            primitive: wgpu::PrimitiveState {
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: None,
            multisample: Default::default(),
            fragment: Some(FragmentState {
                shader: SCREENSHOT_SHADER_HANDLE,
                entry_point: Cow::Borrowed("fs_main"),
                shader_defs: vec![],
                targets: vec![Some(wgpu::ColorTargetState {
                    format: key,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            push_constant_ranges: Vec::new(),
        }
    }
}

pub(crate) fn submit_screenshot_commands(world: &World, encoder: &mut CommandEncoder) {
    let targets = world.resource::<RenderScreenshotTargets>();
    let prepared = world.resource::<RenderScreenshotsPrepared>();
    let pipelines = world.resource::<PipelineCache>();
    let gpu_images = world.resource::<RenderAssets<GpuImage>>();
    let windows = world.resource::<ExtractedWindows>();
    let manual_texture_views = world.resource::<ManualTextureViews>();

    for (entity, render_target) in targets.iter() {
        match render_target {
            NormalizedRenderTarget::Window(window) => {
                let window = window.entity();
                let Some(window) = windows.get(&window) else {
                    continue;
                };
                let width = window.physical_width;
                let height = window.physical_height;
                let Some(texture_format) = window.swap_chain_texture_format else {
                    continue;
                };
                let Some(swap_chain_texture) = window.swap_chain_texture.as_ref() else {
                    continue;
                };
                let texture_view = swap_chain_texture.texture.create_view(&Default::default());
                render_screenshot(
                    encoder,
                    prepared,
                    pipelines,
                    entity,
                    width,
                    height,
                    texture_format,
                    &texture_view,
                );
            }
            NormalizedRenderTarget::Image(image) => {
                let Some(gpu_image) = gpu_images.get(image) else {
                    warn!("Unknown image for screenshot, skipping: {:?}", image);
                    continue;
                };
                let width = gpu_image.size.x;
                let height = gpu_image.size.y;
                let texture_format = gpu_image.texture_format;
                let texture_view = gpu_image.texture_view.deref();
                render_screenshot(
                    encoder,
                    prepared,
                    pipelines,
                    entity,
                    width,
                    height,
                    texture_format,
                    texture_view,
                );
            }
            NormalizedRenderTarget::TextureView(texture_view) => {
                let Some(texture_view) = manual_texture_views.get(texture_view) else {
                    warn!(
                        "Unknown manual texture view for screenshot, skipping: {:?}",
                        texture_view
                    );
                    continue;
                };
                let width = texture_view.size.x;
                let height = texture_view.size.y;
                let texture_format = texture_view.format;
                let texture_view = texture_view.texture_view.deref();
                render_screenshot(
                    encoder,
                    prepared,
                    pipelines,
                    entity,
                    width,
                    height,
                    texture_format,
                    texture_view,
                );
            }
        };
    }
}

#[allow(clippy::too_many_arguments)]
fn render_screenshot(
    encoder: &mut CommandEncoder,
    prepared: &RenderScreenshotsPrepared,
    pipelines: &PipelineCache,
    entity: &Entity,
    width: u32,
    height: u32,
    texture_format: TextureFormat,
    texture_view: &wgpu::TextureView,
) {
    if let Some(prepared_state) = &prepared.get(entity) {
        encoder.copy_texture_to_buffer(
            prepared_state.texture.as_image_copy(),
            wgpu::ImageCopyBuffer {
                buffer: &prepared_state.buffer,
                layout: layout_data(width, height, texture_format),
            },
            Extent3d {
                width,
                height,
                ..Default::default()
            },
        );

        if let Some(pipeline) = pipelines.get_render_pipeline(prepared_state.pipeline_id) {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("screenshot_to_screen_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: texture_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, &prepared_state.bind_group, &[]);
            pass.draw(0..3, 0..1);
        }
    }
}

pub(crate) fn collect_screenshots(world: &mut World) {
    #[cfg(feature = "trace")]
    let _span = bevy_utils::tracing::info_span!("collect_screenshots").entered();

    let sender = world.resource::<RenderScreenshotsSender>().deref().clone();
    let prepared = world.resource::<RenderScreenshotsPrepared>();

    for (entity, prepared) in prepared.iter() {
        let entity = *entity;
        let sender = sender.clone();
        let width = prepared.size.width;
        let height = prepared.size.height;
        let texture_format = prepared.texture.format();
        let pixel_size = texture_format.pixel_size();
        let buffer = prepared.buffer.clone();

        let finish = async move {
            let (tx, rx) = async_channel::bounded(1);
            let buffer_slice = buffer.slice(..);
            // The polling for this map call is done every frame when the command queue is submitted.
            buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
                let err = result.err();
                if err.is_some() {
                    panic!("{}", err.unwrap().to_string());
                }
                tx.try_send(()).unwrap();
            });
            rx.recv().await.unwrap();
            let data = buffer_slice.get_mapped_range();
            // we immediately move the data to CPU memory to avoid holding the mapped view for long
            let mut result = Vec::from(&*data);
            drop(data);

            if result.len() != ((width * height) as usize * pixel_size) {
                // Our buffer has been padded because we needed to align to a multiple of 256.
                // We remove this padding here
                let initial_row_bytes = width as usize * pixel_size;
                let buffered_row_bytes = align_byte_size(width * pixel_size as u32) as usize;

                let mut take_offset = buffered_row_bytes;
                let mut place_offset = initial_row_bytes;
                for _ in 1..height {
                    result.copy_within(take_offset..take_offset + buffered_row_bytes, place_offset);
                    take_offset += buffered_row_bytes;
                    place_offset += initial_row_bytes;
                }
                result.truncate(initial_row_bytes * height as usize);
            }

            if let Err(e) = sender.send((
                entity,
                Image::new(
                    Extent3d {
                        width,
                        height,
                        depth_or_array_layers: 1,
                    },
                    wgpu::TextureDimension::D2,
                    result,
                    texture_format,
                    RenderAssetUsages::RENDER_WORLD,
                ),
            )) {
                error!("Failed to send screenshot: {:?}", e);
            }
        };

        AsyncComputeTaskPool::get().spawn(finish).detach();
    }
}
