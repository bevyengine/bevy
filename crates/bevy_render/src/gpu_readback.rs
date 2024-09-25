use crate::{
    extract_component::ExtractComponentPlugin,
    prelude::Image,
    render_asset::RenderAssets,
    render_resource::{
        Buffer, BufferUsages, Extent3d, ImageDataLayout, SpecializedRenderPipelines, Texture,
        TextureFormat, TextureView,
    },
    renderer::{render_system, RenderDevice},
    storage::{GpuShaderStorageBuffer, ShaderStorageBuffer},
    texture::{GpuImage, TextureFormatPixelInfo},
    Extract, ExtractSchedule, MainWorld, Render, RenderApp, RenderSet,
};
use async_channel::{Receiver, Sender};
use bevy_app::{App, Plugin};
use bevy_asset::Handle;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    change_detection::ResMut,
    entity::Entity,
    event::Event,
    prelude::{Component, Resource, World},
    query::With,
    system::{Commands, Query, Res},
};
use bevy_reflect::Reflect;
use bevy_render_macros::ExtractComponent;
use bevy_utils::{default, tracing::warn, HashMap};
use wgpu::{CommandEncoder, COPY_BYTES_PER_ROW_ALIGNMENT};

const MAX_UNUSED_FRAMES: usize = 3;

/// A component that marks an entity for gpu readback.
///
/// The entity must also have a `Handle<Image>` or `Handle<ShaderStorageBuffer>` component, which
/// will be read back asynchronously to the cpu and trigger a [`ReadbackComplete`] observer.
#[derive(Component, ExtractComponent, Clone, Debug, Default)]
pub struct Readback;

/// An event that is triggered when a gpu readback is complete.
///
/// The event contains the data as a `Vec<u8>`, which can be interpreted as the raw bytes of the
/// read back buffer.
#[derive(Event, Deref, DerefMut, Reflect, Debug)]
#[reflect(Debug)]
pub struct ReadbackComplete(pub Vec<u8>);

#[derive(Resource, Default)]
struct GpuReadbackBufferPool {
    // Map of buffer size to list of buffers, with a flag for whether the buffer is taken and how
    // many frames it has been unused for.
    // TODO: We could ideally write all readback data to one big buffer per frame, the assumption
    // here is that very few entities well actually be read back at once, and their size is
    // unlikely to change.
    buffers: HashMap<u64, Vec<(Buffer, bool, usize)>>, // (Buffer, taken, frames_unused)
}

impl GpuReadbackBufferPool {
    fn get(&mut self, render_device: &RenderDevice, size: u64) -> Buffer {
        let buffers = self.buffers.entry(size).or_insert_with(|| Vec::new());

        // find an untaken buffer for this size
        if let Some((buffer, taken, _)) = buffers.iter_mut().find(|(_, taken, _)| !*taken) {
            *taken = true;
            return buffer.clone();
        }

        let buffer = render_device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Readback Buffer"),
            size,
            usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        buffers.push((buffer.clone(), true, 0));
        buffer
    }

    fn return_buffer(&mut self, buffer: &Buffer) {
        let size = buffer.size() as u64;
        let buffers = self
            .buffers
            .get_mut(&size)
            .expect("Returned buffer of untracked size");
        if let Some((_, taken, _)) = buffers.iter_mut().find(|(b, _, _)| b.id() == buffer.id()) {
            *taken = false;
        } else {
            warn!("Returned buffer that was not allocated");
        }
    }

    fn update(&mut self) {
        for (_, buffers) in &mut self.buffers {
            // Tick all the buffers
            for (_, taken, frames_unused) in &mut *buffers {
                if !*taken {
                    *frames_unused += 1;
                }
            }

            // Remove buffers that haven't been used for MAX_UNUSED_FRAMES
            buffers.retain(|(_, _, frames_unused)| *frames_unused < MAX_UNUSED_FRAMES);
        }
    }
}

enum ReadbackSource {
    Texture {
        texture: Texture,
        layout: ImageDataLayout,
        size: Extent3d,
    },
    Buffer {
        src_start: u64,
        dst_start: u64,
        buffer: Buffer,
    },
}

#[derive(Resource, Default)]
struct GpuReadbacks {
    requested: Vec<GpuReadback>,
    mapped: Vec<GpuReadback>,
}

struct GpuReadback {
    pub entity: Entity,
    pub src: ReadbackSource,
    pub buffer: Buffer,
    pub rx: Receiver<(Entity, Buffer)>,
    pub tx: Sender<(Entity, Buffer)>,
}

pub struct GpuReadbackPlugin;

impl Plugin for GpuReadbackPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ExtractComponentPlugin::<Readback>::default());

        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .init_resource::<GpuReadbackBufferPool>()
                .init_resource::<GpuReadbacks>()
                .add_systems(
                    ExtractSchedule,
                    (extract_readbacks, sync_readbacks).ambiguous_with_all(),
                )
                .add_systems(
                    Render,
                    (
                        prepare_buffers.in_set(RenderSet::PrepareResources),
                        after_render.after(render_system).in_set(RenderSet::Render),
                    ),
                );
        }
    }
}

fn extract_readbacks(
    mut commands: Commands,
    query: Extract<
        Query<
            (
                Entity,
                Option<&Handle<Image>>,
                Option<&Handle<ShaderStorageBuffer>>,
            ),
            With<Readback>,
        >,
    >,
) {
    for (entity, maybe_image, maybe_buffer) in query.iter() {
        if let Some(image) = maybe_image {
            commands.get_or_spawn(entity).insert(image.clone());
        }
        if let Some(buffer) = maybe_buffer {
            commands.get_or_spawn(entity).insert(buffer.clone());
        }
    }
}

fn sync_readbacks(
    mut main_world: ResMut<MainWorld>,
    mut buffer_pool: ResMut<GpuReadbackBufferPool>,
    mut readbacks: ResMut<GpuReadbacks>,
) {
    readbacks.mapped.retain(|readback| {
        if let Ok((entity, buffer)) = readback.rx.try_recv() {
            let buffer_slice = buffer.slice(..);
            let data = buffer_slice.get_mapped_range();
            let result = Vec::from(&*data);
            drop(data);
            main_world.trigger_targets(ReadbackComplete(result), entity);
            buffer_pool.return_buffer(&buffer);
            buffer.unmap();
            false
        } else {
            true
        }
    });

    buffer_pool.update();
}

fn prepare_buffers(
    render_device: Res<RenderDevice>,
    mut readbacks: ResMut<GpuReadbacks>,
    mut buffer_pool: ResMut<GpuReadbackBufferPool>,
    gpu_images: Res<RenderAssets<GpuImage>>,
    ssbos: Res<RenderAssets<GpuShaderStorageBuffer>>,
    handles: Query<
        (
            Entity,
            Option<&Handle<Image>>,
            Option<&Handle<ShaderStorageBuffer>>,
        ),
        With<Readback>,
    >,
) {
    for (entity, maybe_image, maybe_buffer) in handles.iter() {
        if let Some(image) = maybe_image {
            if let Some(gpu_image) = gpu_images.get(image) {
                let size = Extent3d {
                    width: gpu_image.size.x,
                    height: gpu_image.size.y,
                    ..default()
                };
                let layout = layout_data(size.width, size.height, gpu_image.texture_format);
                let buffer = buffer_pool.get(
                    &render_device,
                    get_aligned_size(
                        size.width,
                        size.height,
                        gpu_image.texture_format.pixel_size() as u32,
                    ) as u64,
                );
                let (tx, rx) = async_channel::bounded(1);
                readbacks.requested.push(GpuReadback {
                    entity,
                    src: ReadbackSource::Texture {
                        texture: gpu_image.texture.clone(),
                        layout,
                        size,
                    },
                    buffer,
                    rx,
                    tx,
                });
            }
        }
        if let Some(buffer) = maybe_buffer {
            if let Some(ssbo) = ssbos.get(buffer) {
                let size = ssbo.buffer.size() as u64;
                let buffer = buffer_pool.get(&render_device, size);
                let (tx, rx) = async_channel::bounded(1);
                readbacks.requested.push(GpuReadback {
                    entity,
                    src: ReadbackSource::Buffer {
                        src_start: 0,
                        dst_start: 0,
                        buffer: ssbo.buffer.clone(),
                    },
                    buffer,
                    rx,
                    tx,
                });
            }
        }
    }
}

pub(crate) fn submit_readback_commands(world: &World, command_encoder: &mut CommandEncoder) {
    let readbacks = world.resource::<GpuReadbacks>();
    for readback in &readbacks.requested {
        match &readback.src {
            ReadbackSource::Texture {
                texture,
                layout,
                size,
            } => {
                command_encoder.copy_texture_to_buffer(
                    texture.as_image_copy(),
                    wgpu::ImageCopyBuffer {
                        buffer: &readback.buffer,
                        layout: *layout,
                    },
                    *size,
                );
            }
            ReadbackSource::Buffer {
                src_start,
                dst_start,
                buffer,
            } => {
                command_encoder.copy_buffer_to_buffer(
                    &buffer,
                    *src_start,
                    &readback.buffer,
                    *dst_start,
                    buffer.size(),
                );
            }
        }
    }
}

fn after_render(mut readbacks: ResMut<GpuReadbacks>) {
    // Move requested readbacks to mapped readbacks after submit
    let requested = readbacks.requested.drain(..).collect::<Vec<GpuReadback>>();
    for readback in requested {
        let slice = readback.buffer.slice(..);
        let entity = readback.entity;
        let buffer = readback.buffer.clone();
        let tx = readback.tx.clone();
        slice.map_async(wgpu::MapMode::Read, move |res| {
            res.expect("Failed to map buffer");
            if let Err(e) = tx.try_send((entity, buffer)) {
                warn!("Failed to send readback result: {:?}", e);
            }
        });
        readbacks.mapped.push(readback);
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
