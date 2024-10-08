use crate::{
    extract_component::ExtractComponentPlugin,
    prelude::Image,
    render_asset::RenderAssets,
    render_resource::{Buffer, BufferUsages, Extent3d, ImageDataLayout, Texture, TextureFormat},
    renderer::{render_system, RenderDevice},
    storage::{GpuShaderStorageBuffer, ShaderStorageBuffer},
    texture::{GpuImage, TextureFormatPixelInfo},
    ExtractSchedule, MainWorld, Render, RenderApp, RenderSet,
};
use async_channel::{Receiver, Sender};
use bevy_app::{App, Plugin};
use bevy_asset::Handle;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::schedule::IntoSystemConfigs;
use bevy_ecs::{
    change_detection::ResMut,
    entity::Entity,
    event::Event,
    prelude::{Component, Resource, World},
    system::{Query, Res},
};
use bevy_reflect::Reflect;
use bevy_render_macros::ExtractComponent;
use bevy_utils::{default, tracing::warn, HashMap};
use encase::internal::ReadFrom;
use encase::private::Reader;
use encase::ShaderType;
use wgpu::{CommandEncoder, COPY_BYTES_PER_ROW_ALIGNMENT};

/// A plugin that enables reading back gpu buffers and textures to the cpu.
pub struct GpuReadbackPlugin {
    /// Describes the number of frames a buffer can be unused before it is removed from the pool in
    /// order to avoid unnecessary reallocations.
    max_unused_frames: usize,
}

impl Default for GpuReadbackPlugin {
    fn default() -> Self {
        Self {
            max_unused_frames: 10,
        }
    }
}

impl Plugin for GpuReadbackPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ExtractComponentPlugin::<Readback>::default());

        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .init_resource::<GpuReadbackBufferPool>()
                .init_resource::<GpuReadbacks>()
                .insert_resource(GpuReadbackMaxUnusedFrames(self.max_unused_frames))
                .add_systems(ExtractSchedule, sync_readbacks.ambiguous_with_all())
                .add_systems(
                    Render,
                    (
                        prepare_buffers.in_set(RenderSet::PrepareResources),
                        map_buffers.after(render_system).in_set(RenderSet::Render),
                    ),
                );
        }
    }
}

/// A component that registers the wrapped handle for gpu readback, either a texture or a buffer.
///
/// Data is read asynchronously and will be triggered on the entity via the [`ReadbackComplete`] event
/// when complete. If this component is not removed, the readback will be attempted every frame
#[derive(Component, ExtractComponent, Clone, Debug)]
pub enum Readback {
    Texture(Handle<Image>),
    Buffer(Handle<ShaderStorageBuffer>),
}

impl Readback {
    /// Create a readback component for a texture using the given handle.
    pub fn texture(image: Handle<Image>) -> Self {
        Self::Texture(image)
    }

    /// Create a readback component for a buffer using the given handle.
    pub fn buffer(buffer: Handle<ShaderStorageBuffer>) -> Self {
        Self::Buffer(buffer)
    }
}

/// An event that is triggered when a gpu readback is complete.
///
/// The event contains the data as a `Vec<u8>`, which can be interpreted as the raw bytes of the
/// requested buffer or texture.
#[derive(Event, Deref, DerefMut, Reflect, Debug)]
#[reflect(Debug)]
pub struct ReadbackComplete(pub Vec<u8>);

impl ReadbackComplete {
    /// Convert the raw bytes of the event to a shader type.
    pub fn to_shader_type<T: ShaderType + ReadFrom + Default>(&self) -> T {
        let mut val = T::default();
        let mut reader = Reader::new::<T>(&self.0, 0).expect("Failed to create Reader");
        T::read_from(&mut val, &mut reader);
        val
    }
}

#[derive(Resource)]
struct GpuReadbackMaxUnusedFrames(usize);

struct GpuReadbackBuffer {
    buffer: Buffer,
    taken: bool,
    frames_unused: usize,
}

#[derive(Resource, Default)]
struct GpuReadbackBufferPool {
    // Map of buffer size to list of buffers, with a flag for whether the buffer is taken and how
    // many frames it has been unused for.
    // TODO: We could ideally write all readback data to one big buffer per frame, the assumption
    // here is that very few entities well actually be read back at once, and their size is
    // unlikely to change.
    buffers: HashMap<u64, Vec<GpuReadbackBuffer>>,
}

impl GpuReadbackBufferPool {
    fn get(&mut self, render_device: &RenderDevice, size: u64) -> Buffer {
        let buffers = self.buffers.entry(size).or_default();

        // find an untaken buffer for this size
        if let Some(buf) = buffers.iter_mut().find(|x| !x.taken) {
            buf.taken = true;
            buf.frames_unused = 0;
            return buf.buffer.clone();
        }

        let buffer = render_device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Readback Buffer"),
            size,
            usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        buffers.push(GpuReadbackBuffer {
            buffer: buffer.clone(),
            taken: true,
            frames_unused: 0,
        });
        buffer
    }

    // Returns the buffer to the pool so it can be used in a future frame
    fn return_buffer(&mut self, buffer: &Buffer) {
        let size = buffer.size();
        let buffers = self
            .buffers
            .get_mut(&size)
            .expect("Returned buffer of untracked size");
        if let Some(buf) = buffers.iter_mut().find(|x| x.buffer.id() == buffer.id()) {
            buf.taken = false;
        } else {
            warn!("Returned buffer that was not allocated");
        }
    }

    fn update(&mut self, max_unused_frames: usize) {
        for (_, buffers) in &mut self.buffers {
            // Tick all the buffers
            for buf in &mut *buffers {
                if !buf.taken {
                    buf.frames_unused += 1;
                }
            }

            // Remove buffers that haven't been used for MAX_UNUSED_FRAMES
            buffers.retain(|x| x.frames_unused < max_unused_frames);
        }

        // Remove empty buffer sizes
        self.buffers.retain(|_, buffers| !buffers.is_empty());
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
    pub rx: Receiver<(Entity, Buffer, Vec<u8>)>,
    pub tx: Sender<(Entity, Buffer, Vec<u8>)>,
}

fn sync_readbacks(
    mut main_world: ResMut<MainWorld>,
    mut buffer_pool: ResMut<GpuReadbackBufferPool>,
    mut readbacks: ResMut<GpuReadbacks>,
    max_unused_frames: Res<GpuReadbackMaxUnusedFrames>,
) {
    readbacks.mapped.retain(|readback| {
        if let Ok((entity, buffer, result)) = readback.rx.try_recv() {
            main_world.trigger_targets(ReadbackComplete(result), entity);
            buffer_pool.return_buffer(&buffer);
            false
        } else {
            true
        }
    });

    buffer_pool.update(max_unused_frames.0);
}

fn prepare_buffers(
    render_device: Res<RenderDevice>,
    mut readbacks: ResMut<GpuReadbacks>,
    mut buffer_pool: ResMut<GpuReadbackBufferPool>,
    gpu_images: Res<RenderAssets<GpuImage>>,
    ssbos: Res<RenderAssets<GpuShaderStorageBuffer>>,
    handles: Query<(Entity, &Readback)>,
) {
    for (entity, readback) in handles.iter() {
        match readback {
            Readback::Texture(image) => {
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
            Readback::Buffer(buffer) => {
                if let Some(ssbo) = ssbos.get(buffer) {
                    let size = ssbo.buffer.size();
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
                    buffer,
                    *src_start,
                    &readback.buffer,
                    *dst_start,
                    buffer.size(),
                );
            }
        }
    }
}

/// Move requested readbacks to mapped readbacks after commands have been submitted in render system
fn map_buffers(mut readbacks: ResMut<GpuReadbacks>) {
    let requested = readbacks.requested.drain(..).collect::<Vec<GpuReadback>>();
    for readback in requested {
        let slice = readback.buffer.slice(..);
        let entity = readback.entity;
        let buffer = readback.buffer.clone();
        let tx = readback.tx.clone();
        slice.map_async(wgpu::MapMode::Read, move |res| {
            res.expect("Failed to map buffer");
            let buffer_slice = buffer.slice(..);
            let data = buffer_slice.get_mapped_range();
            let result = Vec::from(&*data);
            drop(data);
            buffer.unmap();
            if let Err(e) = tx.try_send((entity, buffer, result)) {
                warn!("Failed to send readback result: {:?}", e);
            }
        });
        readbacks.mapped.push(readback);
    }
}

// Utils

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
