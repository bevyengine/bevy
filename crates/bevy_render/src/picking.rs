//! Gpu picking let's you know which entity is currently being rendered under the mouse.
//!
//! # How this works:
//!
//! This happens over multiple frames
//! - Frame N:
//!     - For each visible mesh, generate a mesh id.
//!     - For each mesh being rendered, output it's mesh id to a texture.
//!     - Once everything is rendered copy that texture to the cpu
//! - Frame N + 1:
//!     - Map the mesh id buffer and send it to the main world.
//!     - This step will poll the gpu if necessary, so it could block here
//! - Frame N + 2:
//!     - From the main world you can give it a position like the current mouse position and
//!       know exactly which entity was rendered at that specific screen location.
//!       Since this takes multiple frames, the exact entity under the mouse might not be
//!       the same as the one visible.
//!
//! - This works at the `Camera` level, so it will work with multiple windows or split-screen.
//!
//! # Api Overview:
//!
//! To enable the feature, you need to add the [`GpuPickingPlugin`]. You then need to add
//! the [`GpuPickingCamera`] to any `Camera` that will be used for picking. Then add the
//! [`GpuPickingMesh`] comnponent to any `Mesh` that will need to be picked.
//!
//! Once those components are added, you can query for [`GpuPickingCamera`]
//! and use `GpuPickingCamera::get_entity(position)` to know which entity is at the
//! given position on screen
//!
//! # Warning
//!
//! The mesh id generated every frame is currently encoded as a u16. This means you can
//! only have up to 65536 _visible_ entities on screen.

use crate::{
    camera::ExtractedCamera,
    extract_component::{ExtractComponent, ExtractComponentPlugin},
    render_resource::{Buffer, Texture},
    renderer::RenderDevice,
    texture::{CachedTexture, TextureFormatPixelInfo},
    Render, RenderApp, RenderSet,
};
use async_channel::{Receiver, Sender};
use bevy_app::{Plugin, PreUpdate};
use bevy_ecs::{prelude::*, query::QueryItem};

use bevy_math::UVec2;
use bevy_utils::{default, HashMap};
use wgpu::{
    BufferDescriptor, BufferUsages, Color, CommandEncoder, Extent3d, ImageDataLayout, MapMode,
    Operations, RenderPassColorAttachment, TextureFormat,
};

pub const MESH_ID_TEXTURE_FORMAT: TextureFormat = TextureFormat::R16Uint;

const BUFFER_COUNT: usize = 2;

/// This plugin enables the gpu picking feature of bevy.
pub struct GpuPickingPlugin;
impl Plugin for GpuPickingPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_plugins((
            ExtractComponentPlugin::<GpuPickingMesh>::default(),
            ExtractComponentPlugin::<GpuPickingCamera>::default(),
        ))
        // WARN It's really important for this to run in PreUpdate,
        // otherwise this might introduce another frame delay to picking
        .add_systems(PreUpdate, receive_buffer);

        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .insert_resource(CurrentGpuPickingBufferIndex(0))
            .add_systems(
                Render,
                (
                    prepare_buffers.in_set(RenderSet::PrepareResources),
                    send_buffer.in_set(RenderSet::RenderFlush),
                    increment_index.in_set(RenderSet::Cleanup),
                ),
            );
    }
}

/// Gpu picking uses a double buffer technique, this index is used to know which buffer should be used this frame
#[derive(Resource)]
pub struct CurrentGpuPickingBufferIndex(usize);
fn increment_index(mut curr_index: ResMut<CurrentGpuPickingBufferIndex>) {
    curr_index.0 = (curr_index.0 + 1) % BUFFER_COUNT;
}

/// Marker component to indicate that a mesh should be available for gpu picking
#[derive(Component, ExtractComponent, Clone)]
pub struct GpuPickingMesh;

/// This component is used to indicate if a camera should support gpu picking.
/// Any mesh with the [`GpuPickingMesh`] component that is visible from this camera
/// will be pickable.
#[derive(Component)]
pub struct GpuPickingCamera {
    /// Used to send the required data between the main world and render world
    data_channel: (Sender<GpuPickingData>, Receiver<GpuPickingData>),
    /// The latest picking data received
    data: GpuPickingData,
    /// Used to determine if the buffer is mapped
    /// This is only used in the render world, but this is here to avoid needlessly creating a new channel every frame
    map_status_channel: (Sender<()>, Receiver<()>),
}

impl Default for GpuPickingCamera {
    fn default() -> Self {
        Self::new()
    }
}

impl GpuPickingCamera {
    pub fn new() -> Self {
        Self {
            data_channel: async_channel::bounded(1),
            data: GpuPickingData::default(),
            map_status_channel: async_channel::bounded(1),
        }
    }

    /// Get the entity at the given position.
    /// If there is no entity, returns `None`.
    pub fn get_entity(&self, pos: UVec2) -> Option<Entity> {
        // We know the position, but in order to find the true position of the bytes
        // we're interested in, we have to know how wide a single row in the GPU written buffer is.
        // Due to alignment requirements this may be wider than the physical camera size because
        // of padding.
        let pixel_size = MESH_ID_TEXTURE_FORMAT.pixel_size();
        let start =
            (pos.y as usize * self.data.padded_bytes_per_row) + (pos.x as usize * pixel_size);
        let end = start + pixel_size;
        if end > self.data.mesh_id_buffer.len() {
            return None;
        }

        // TODO This is currently a constant, but could be user configurable
        let texture_bytes = &self.data.mesh_id_buffer[start..end];
        let index = match MESH_ID_TEXTURE_FORMAT {
            TextureFormat::R16Uint => u16::from_ne_bytes(texture_bytes.try_into().ok()?) as usize,
            TextureFormat::R32Uint => u32::from_ne_bytes(texture_bytes.try_into().ok()?) as usize,
            _ => panic!("Unsupported mesh id texture format"),
        };
        let entity = self.data.visible_mesh_entities[index];

        if entity != Entity::PLACEHOLDER {
            Some(entity)
        } else {
            None
        }
    }
}

impl ExtractComponent for GpuPickingCamera {
    type Query = &'static Self;
    type Filter = ();
    type Out = ExtractedGpuPickingCamera;
    fn extract_component(picking_camera: QueryItem<'_, Self::Query>) -> Option<Self::Out> {
        let (sender, _) = picking_camera.data_channel.clone();
        Some(ExtractedGpuPickingCamera {
            buffers: None,
            sender,
            map_status_channel: picking_camera.map_status_channel.clone(),
        })
    }
}

/// Data needed in the render world to manage the entity buffer
#[derive(Component)]
pub struct ExtractedGpuPickingCamera {
    buffers: Option<GpuPickingCameraBuffers>,
    sender: Sender<GpuPickingData>,
    map_status_channel: (Sender<()>, Receiver<()>),
}

impl ExtractedGpuPickingCamera {
    /// Runs all the operation for the node
    /// This needs to be here because it needs a dependency on wgpu and `bevy_core_pipeline` doens't have it.
    pub fn run_node(
        &self,
        encoder: &mut CommandEncoder,
        texture: &Texture,
        current_buffer_index: &CurrentGpuPickingBufferIndex,
    ) {
        let Some(buffers) = self.buffers.as_ref() else {
            return;
        };

        // Copy current frame to next buffer
        let copy_index = (current_buffer_index.0 + 1) % BUFFER_COUNT;
        buffers.copy_texture_to_buffer(encoder, texture, copy_index);

        // Map current buffer that will be copied and sent after the graph has finished
        let map_index = current_buffer_index.0;
        let buffer_slice = buffers.entity_buffers[map_index].slice(..);
        let (tx, _) = self.map_status_channel.clone();
        buffer_slice.map_async(MapMode::Read, move |result| match result {
            Ok(_) => tx.try_send(()).unwrap(),
            Err(err) => panic!("Failed to map entity buffer {map_index}: {err}"),
        });
    }
}

/// Data sent between the render world and main world
#[derive(Default)]
struct GpuPickingData {
    /// Padding required to compute the entity with the exact position in the buffer
    padded_bytes_per_row: usize,
    /// Buffer representing the entity texture
    mesh_id_buffer: Vec<u8>,
    /// A list of the visible entities during the frame the buffer was generated
    /// The buffer contains an index into this list
    visible_mesh_entities: Vec<Entity>,
}

/// Contains the buffers and their dimensions required for gpu picking
#[derive(Clone)]
struct GpuPickingCameraBuffers {
    entity_buffers: [Buffer; BUFFER_COUNT],
    // All buffers have the same dimension so we only need one
    buffer_dimensions: BufferDimensions,
}

impl GpuPickingCameraBuffers {
    /// Copies the given texture to the buffer at the given index
    fn copy_texture_to_buffer(
        &self,
        encoder: &mut CommandEncoder,
        texture: &Texture,
        buffer_index: usize,
    ) {
        encoder.copy_texture_to_buffer(
            texture.as_image_copy(),
            wgpu::ImageCopyBuffer {
                buffer: &self.entity_buffers[buffer_index],
                layout: ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(self.buffer_dimensions.padded_bytes_per_row as u32),
                    rows_per_image: None,
                },
            },
            Extent3d {
                width: self.buffer_dimensions.width as u32,
                height: self.buffer_dimensions.height as u32,
                ..default()
            },
        );
    }
}

/// This is created every frame and contains a list of the currently visible entities
#[derive(Resource)]
pub struct VisibleMeshEntities(pub Option<Vec<Entity>>);

/// Sends the mesh id buffer to the main world
fn send_buffer(
    query: Query<&ExtractedGpuPickingCamera>,
    render_device: Res<RenderDevice>,
    mut visible_mesh_entities: ResMut<VisibleMeshEntities>,
    current_buffer_index: Res<CurrentGpuPickingBufferIndex>,
) {
    let Some(visible_mesh_entities) = visible_mesh_entities.0.take() else {
        return;
    };

    for gpu_picking_camera in &query {
        let Some(buffers) = gpu_picking_camera.buffers.as_ref() else {
            return;
        };

        // We need to make sure the map_async has completed before reading it
        let (_, rx) = gpu_picking_camera.map_status_channel.clone();
        if rx.try_recv().is_err() {
            // Sometimes the map isn't done at this point so we need to poll the gpu
            // This will block until the map is done
            render_device.poll(wgpu::MaintainBase::Wait);
            // This is to empty the channel before we continue
            rx.try_recv().expect("map_async should have been completed");
        }

        let send_index = current_buffer_index.0;
        let buffer_slice = buffers.entity_buffers[send_index].slice(..);
        let buffer_view = buffer_slice.get_mapped_range();
        let mesh_id_buffer = buffer_view.to_vec();
        // We have to make sure all mapped views are dropped before we unmap the buffer.
        drop(buffer_view);
        // We need to unmap the buffer because it will be used in the next frame and can't be mapped at that point
        buffers.entity_buffers[send_index].unmap();

        // Send the data to the main world
        if let Err(err) = gpu_picking_camera.sender.try_send(GpuPickingData {
            padded_bytes_per_row: buffers.buffer_dimensions.padded_bytes_per_row,
            mesh_id_buffer,
            visible_mesh_entities: visible_mesh_entities.clone(),
        }) {
            bevy_log::error!("Failed to send entity buffer: {err}");
        }
    }
}

/// Receives the mesh id buffer from the render world
fn receive_buffer(mut cameras: Query<&mut GpuPickingCamera>) {
    for mut cam in &mut cameras {
        let (_, receiver) = cam.data_channel.clone();
        let Ok(data) = receiver.try_recv() else {
            continue;
        };
        cam.data = data;
    }
}

/// The textures used to draw the entity for each rendered mesh
#[derive(Component, Clone)]
pub struct VisibleMeshIdTextures {
    pub main: CachedTexture,
    pub sampled: Option<CachedTexture>,
}

impl VisibleMeshIdTextures {
    /// This is the color that will represent "no entity" in the mesh id buffer
    pub fn clear_color() -> wgpu::Color {
        Color {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 0.0,
        }
    }

    /// Retrieve this target's color attachment. This will use [`Self::sampled`] and resolve to [`Self::main`] if
    /// the target has sampling enabled. Otherwise it will use [`Self::main`] directly.
    pub fn get_color_attachment(&self, ops: Operations<wgpu::Color>) -> RenderPassColorAttachment {
        match &self.sampled {
            Some(sampled_texture) => RenderPassColorAttachment {
                view: &sampled_texture.default_view,
                resolve_target: Some(&self.main.default_view),
                ops,
            },
            None => RenderPassColorAttachment {
                view: &self.main.default_view,
                resolve_target: None,
                ops,
            },
        }
    }
}

/// This creates the required buffers for each camera
fn prepare_buffers(
    render_device: Res<RenderDevice>,
    mut cameras: Query<
        (Entity, &ExtractedCamera, &mut ExtractedGpuPickingCamera),
        Changed<ExtractedCamera>,
    >,
    mut buffer_cache: Local<HashMap<Entity, (BufferDimensions, [Buffer; BUFFER_COUNT])>>,
) {
    for (entity, camera, mut gpu_picking_camera) in &mut cameras {
        let Some(size) = camera.physical_target_size else {
            continue;
        };

        // We only want to create a buffer when there's no buffers in the cache
        // or when the dimensions don't match
        let mut create_buffer = true;
        if let Some((buffer_dimensions, _)) = buffer_cache.get(&entity) {
            // We could potentially account for padding and only re-create buffers
            // when the full size of the buffer doesn't match
            create_buffer = buffer_dimensions.width != size.x as usize
                || buffer_dimensions.height != size.y as usize;
        }

        if create_buffer {
            let buffer_dimensions =
                BufferDimensions::new(size.x as usize, size.y as usize, MESH_ID_TEXTURE_FORMAT);
            let desc = BufferDescriptor {
                label: None,
                size: buffer_dimensions.size() as u64,
                usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
                mapped_at_creation: false,
            };
            let entity_buffers = [
                render_device.create_buffer(&BufferDescriptor {
                    label: Some("Entity Buffer 0"),
                    ..desc
                }),
                render_device.create_buffer(&BufferDescriptor {
                    label: Some("Entity Buffer 1"),
                    ..desc
                }),
            ];
            buffer_cache.insert(entity, (buffer_dimensions, entity_buffers));
        }

        let (buffer_dimensions, buffers) = buffer_cache
            .get(&entity)
            .expect("Buffers should have been created already");
        gpu_picking_camera.buffers = Some(GpuPickingCameraBuffers {
            entity_buffers: buffers.clone(),
            buffer_dimensions: *buffer_dimensions,
        });
    }
}

/// Used to represent the size of a [`Buffer`] and the padding required for each row.
/// We need to know the padding because the rows need to be 256 bit aligned.
///
/// Copied from <https://github.com/gfx-rs/wgpu/blob/dcad7dfba92dd85c3ca21bb553a61834e01b04f5/examples/capture/src/main.rs#L187>
#[derive(Clone, Copy)]
pub struct BufferDimensions {
    width: usize,
    height: usize,
    padded_bytes_per_row: usize,
}

impl BufferDimensions {
    fn new(width: usize, height: usize, texture_format: TextureFormat) -> Self {
        let bytes_per_pixel = texture_format.pixel_size();
        let unpadded_bytes_per_row = width * bytes_per_pixel;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize;
        let padded_bytes_per_row_padding = (align - unpadded_bytes_per_row % align) % align;
        let padded_bytes_per_row = unpadded_bytes_per_row + padded_bytes_per_row_padding;
        Self {
            width,
            height,
            padded_bytes_per_row,
        }
    }

    fn size(&self) -> usize {
        self.padded_bytes_per_row * self.height
    }
}
