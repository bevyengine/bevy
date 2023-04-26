use std::{borrow::Cow, path::Path};

use bevy_app::Plugin;
use bevy_asset::{load_internal_asset, HandleUntyped};
use bevy_ecs::prelude::*;
use bevy_log::{error, info, info_span};
use bevy_reflect::TypeUuid;
use bevy_tasks::AsyncComputeTaskPool;
use bevy_utils::HashMap;
use parking_lot::Mutex;
use thiserror::Error;
use wgpu::{
    CommandEncoder, Extent3d, ImageDataLayout, TextureFormat, COPY_BYTES_PER_ROW_ALIGNMENT,
};

use crate::{
    prelude::{Image, Shader},
    render_resource::{
        BindGroup, BindGroupLayout, Buffer, CachedRenderPipelineId, FragmentState, PipelineCache,
        RenderPipelineDescriptor, SpecializedRenderPipeline, SpecializedRenderPipelines, Texture,
        VertexState,
    },
    renderer::RenderDevice,
    texture::TextureFormatPixelInfo,
    RenderApp,
};

use super::ExtractedWindows;

pub type ScreenshotFn = Box<dyn FnOnce(Image) + Send + Sync>;

/// A resource which allows for taking screenshots of the window.
#[derive(Resource, Default)]
pub struct ScreenshotManager {
    // this is in a mutex to enable extraction with only an immutable reference
    pub(crate) callbacks: Mutex<HashMap<Entity, ScreenshotFn>>,
}

#[derive(Error, Debug)]
#[error("A screenshot for this window has already been requested.")]
pub struct ScreenshotAlreadyRequestedError;

impl ScreenshotManager {
    /// Signals the renderer to take a screenshot of this frame.
    ///
    /// The given callback will eventually be called on one of the [`AsyncComputeTaskPool`]s threads.
    pub fn take_screenshot(
        &mut self,
        window: Entity,
        callback: impl FnOnce(Image) + Send + Sync + 'static,
    ) -> Result<(), ScreenshotAlreadyRequestedError> {
        self.callbacks
            .get_mut()
            .try_insert(window, Box::new(callback))
            .map(|_| ())
            .map_err(|_| ScreenshotAlreadyRequestedError)
    }

    /// Signals the renderer to take a screenshot of this frame.
    ///
    /// The screenshot will eventually be saved to the given path, and the format will be derived from the extension.
    pub fn save_screenshot_to_disk(
        &mut self,
        window: Entity,
        path: impl AsRef<Path>,
    ) -> Result<(), ScreenshotAlreadyRequestedError> {
        let path = path.as_ref().to_owned();
        self.take_screenshot(window, move |img| match img.try_into_dynamic() {
            Ok(dyn_img) => match image::ImageFormat::from_path(&path) {
                Ok(format) => {
                    // discard the alpha channel which stores brightness values when HDR is enabled to make sure
                    // the screenshot looks right
                    let img = dyn_img.to_rgb8();
                    match img.save_with_format(&path, format) {
                        Ok(_) => info!("Screenshot saved to {}", path.display()),
                        Err(e) => error!("Cannot save screenshot, IO error: {e}"),
                    }
                }
                Err(e) => error!("Cannot save screenshot, requested format not recognized: {e}"),
            },
            Err(e) => error!("Cannot save screenshot, screen format cannot be understood: {e}"),
        })
    }
}

pub struct ScreenshotPlugin;

const SCREENSHOT_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 11918575842344596158);

impl Plugin for ScreenshotPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.init_resource::<ScreenshotManager>();

        load_internal_asset!(
            app,
            SCREENSHOT_SHADER_HANDLE,
            "screenshot.wgsl",
            Shader::from_wgsl
        );

        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.init_resource::<SpecializedRenderPipelines<ScreenshotToScreenPipeline>>();
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

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("screenshot-to-screen-bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: false },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            }],
        });

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
                shader: SCREENSHOT_SHADER_HANDLE.typed(),
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
                unclipped_depth: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(FragmentState {
                shader: SCREENSHOT_SHADER_HANDLE.typed(),
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

pub struct ScreenshotPreparedState {
    pub texture: Texture,
    pub buffer: Buffer,
    pub bind_group: BindGroup,
    pub pipeline_id: CachedRenderPipelineId,
}

pub(crate) fn submit_screenshot_commands(world: &World, encoder: &mut CommandEncoder) {
    let windows = world.resource::<ExtractedWindows>();
    let pipelines = world.resource::<PipelineCache>();

    for window in windows.values() {
        if let Some(memory) = &window.screenshot_memory {
            let width = window.physical_width;
            let height = window.physical_height;
            let texture_format = window.swap_chain_texture_format.unwrap();

            encoder.copy_texture_to_buffer(
                memory.texture.as_image_copy(),
                wgpu::ImageCopyBuffer {
                    buffer: &memory.buffer,
                    layout: crate::view::screenshot::layout_data(width, height, texture_format),
                },
                Extent3d {
                    width,
                    height,
                    ..Default::default()
                },
            );
            if let Some(pipeline) = pipelines.get_render_pipeline(memory.pipeline_id) {
                let true_swapchain_texture_view = window
                    .swap_chain_texture
                    .as_ref()
                    .unwrap()
                    .texture
                    .create_view(&Default::default());
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("screenshot_to_screen_pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &true_swapchain_texture_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: true,
                        },
                    })],
                    depth_stencil_attachment: None,
                });
                pass.set_pipeline(pipeline);
                pass.set_bind_group(0, &memory.bind_group, &[]);
                pass.draw(0..3, 0..1);
            }
        }
    }
}

pub(crate) fn collect_screenshots(world: &mut World) {
    let _span = info_span!("collect_screenshots");

    let mut windows = world.resource_mut::<ExtractedWindows>();
    for window in windows.values_mut() {
        if let Some(screenshot_func) = window.screenshot_func.take() {
            let width = window.physical_width;
            let height = window.physical_height;
            let texture_format = window.swap_chain_texture_format.unwrap();
            let pixel_size = texture_format.pixel_size();
            let ScreenshotPreparedState { buffer, .. } = window.screenshot_memory.take().unwrap();

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
                drop(buffer);

                if result.len() != ((width * height) as usize * pixel_size) {
                    // Our buffer has been padded because we needed to align to a multiple of 256.
                    // We remove this padding here
                    let initial_row_bytes = width as usize * pixel_size;
                    let buffered_row_bytes = align_byte_size(width * pixel_size as u32) as usize;

                    let mut take_offset = buffered_row_bytes;
                    let mut place_offset = initial_row_bytes;
                    for _ in 1..height {
                        result.copy_within(
                            take_offset..take_offset + buffered_row_bytes,
                            place_offset,
                        );
                        take_offset += buffered_row_bytes;
                        place_offset += initial_row_bytes;
                    }
                    result.truncate(initial_row_bytes * height as usize);
                }

                screenshot_func(Image::new(
                    Extent3d {
                        width,
                        height,
                        depth_or_array_layers: 1,
                    },
                    wgpu::TextureDimension::D2,
                    result,
                    texture_format,
                ));
            };

            AsyncComputeTaskPool::get().spawn(finish).detach();
        }
    }
}
