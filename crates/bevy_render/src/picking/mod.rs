use std::{
    mem::size_of,
    sync::{Arc, Mutex},
};

use bevy_ecs::{prelude::*, query::QueryItem};
use bevy_math::{UVec2, Vec2};
use bevy_utils::HashMap;
use bevy_window::CursorMoved;
use wgpu::{
    BufferDescriptor, BufferUsages, BufferView, Extent3d, ImageCopyBuffer, ImageDataLayout,
    Maintain, MapMode, Operations, RenderPassColorAttachment, TextureDescriptor, TextureDimension,
    TextureFormat, TextureUsages,
};

use crate::{
    camera::{Camera, ExtractedCamera},
    extract_component::ExtractComponent,
    prelude::Color,
    render_resource::Buffer,
    renderer::{RenderContext, RenderDevice},
    texture::{CachedTexture, TextureCache},
    view::Msaa,
};

#[cfg(feature = "trace")]
use bevy_utils::tracing::info_span;

pub fn copy_to_buffer(
    camera_size: UVec2,
    picking: &Picking,
    picking_textures: &PickingTextures,
    render_context: &mut RenderContext,
) {
    let picking_buffer_size = PickingBufferSize::from(camera_size);

    let (buffer, size) = picking
        .buffer
        .try_lock()
        .expect("TODO: Can we lock here?")
        .as_ref()
        .expect("Buffer should have been prepared")
        .clone();

    render_context.command_encoder.copy_texture_to_buffer(
        picking_textures.main.texture.as_image_copy(),
        ImageCopyBuffer {
            buffer: &buffer,
            layout: ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(
                    std::num::NonZeroU32::new(size.padded_bytes_per_row as u32).unwrap(),
                ),
                rows_per_image: None,
            },
        },
        Extent3d {
            width: picking_buffer_size.texture_size.x,
            height: picking_buffer_size.texture_size.y,
            depth_or_array_layers: 1,
        },
    );
}

/// Add this to a camera in order for the camera to also render to a buffer
/// with entity indices instead of colors.
#[derive(Component, Debug, Clone, Default)]
pub struct Picking {
    pub buffer: Arc<Mutex<Option<(Buffer, PickingBufferSize)>>>,
}

#[derive(Debug, Clone, Default)]
pub struct PickingBufferSize {
    pub texture_size: UVec2,
    pub padded_bytes_per_row: usize,
}

impl PickingBufferSize {
    pub fn new(width: u32, height: u32) -> Self {
        // See: https://github.com/gfx-rs/wgpu/blob/master/wgpu/examples/capture/main.rs#L193
        let bytes_per_pixel = size_of::<u32>();
        let unpadded_bytes_per_row = width as usize * bytes_per_pixel;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize;
        let padded_bytes_per_row_padding = (align - (unpadded_bytes_per_row % align)) % align;
        let padded_bytes_per_row = unpadded_bytes_per_row + padded_bytes_per_row_padding;

        Self {
            texture_size: UVec2 {
                x: width,
                y: height,
            },
            padded_bytes_per_row,
        }
    }

    pub fn total_needed_bytes(&self) -> u64 {
        (self.texture_size.y as usize * self.padded_bytes_per_row) as u64
    }
}

impl From<Extent3d> for PickingBufferSize {
    fn from(texture_extent: Extent3d) -> Self {
        Self::new(texture_extent.width, texture_extent.height)
    }
}

impl From<UVec2> for PickingBufferSize {
    fn from(texture_extent: UVec2) -> Self {
        Self::new(texture_extent.x, texture_extent.y)
    }
}

impl ExtractComponent for Picking {
    type Query = &'static Self;
    type Filter = With<Camera>;
    type Out = Self;

    fn extract_component(item: QueryItem<'_, Self::Query>) -> Option<Self::Out> {
        Some(item.clone())
    }
}

#[derive(Component, Clone)]
pub struct PickingTextures {
    pub main: CachedTexture,
    pub sampled: Option<CachedTexture>,
}

impl PickingTextures {
    // Same logic as [`ViewTarget`].

    /// The clear color which should be used to clear picking textures.
    /// Picking textures use a single u32 value for each pixel.
    /// This color clears that with `u32::MAX`.
    /// This allows all entity index values below `u32::MAX` to be valid.
    pub fn clear_color() -> wgpu::Color {
        Color::Rgba {
            red: f32::MAX,
            green: f32::MAX,
            blue: f32::MAX,
            alpha: f32::MAX,
        }
        .into()
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

    pub fn get_unsampled_color_attachment(
        &self,
        ops: Operations<wgpu::Color>,
    ) -> RenderPassColorAttachment {
        RenderPassColorAttachment {
            view: &self.main.default_view,
            resolve_target: None,
            ops,
        }
    }
}

#[derive(Debug, Resource, PartialEq, Eq, PartialOrd, Ord)]
pub enum PickedEventVariant {
    /// The given entity is now picked/hovered.
    // TODO: Perhaps it's useful to provide the coords as well?
    Picked,

    /// The given entity is no longer picked/hovered.
    Unpicked,
}

#[derive(Debug, Resource, PartialEq, Eq, PartialOrd, Ord)]
pub struct PickedEvent {
    /// Which entity triggered the event.
    pub entity: Entity,

    /// Which event variant occurred.
    pub event: PickedEventVariant,
}

impl PickedEvent {
    fn new(entity_index: u32, event: PickedEventVariant) -> Self {
        Self {
            entity: Entity::from_raw(entity_index),
            event,
        }
    }

    fn new_picked(entity_index: u32) -> Self {
        Self::new(entity_index, PickedEventVariant::Picked)
    }

    fn new_unpicked(entity_index: u32) -> Self {
        Self::new(entity_index, PickedEventVariant::Unpicked)
    }
}

fn cursor_coords_to_entity_index(
    cursor: Vec2,
    camera_size: UVec2,
    picking_buffer_size: &PickingBufferSize,
    buffer_view: &BufferView,
) -> u32 {
    // The GPU image has a top-left origin,
    // but the cursor has a bottom-left origin.
    // Therefore we must flip the vertical axis.
    let x = cursor.x as usize;
    let y = camera_size.y as usize - cursor.y as usize;

    // We know the coordinates, but in order to find the true position of the 4 bytes
    // we're interested in, we have to know how wide a single line in the GPU written buffer is.
    // Due to alignment requirements this may be wider than the physical camera size because
    // of padding.
    let padded_width = picking_buffer_size.padded_bytes_per_row;

    let pixel_size = std::mem::size_of::<u32>();

    let start = (y * padded_width) + (x * pixel_size);
    let end = start + pixel_size;

    let bytes = &buffer_view[start..end];

    u32::from_le_bytes(bytes.try_into().unwrap())
}

pub fn picking_events(
    // TODO: Must be hashmap to have per-camera
    // Maybe use the entity of the below query for that?
    mut picked: Local<Option<u32>>,

    query: Query<(&Picking, &Camera)>,
    // TODO: Ensure we get events on the same frame as this guy
    // UPDATE: These events are issued via winit, so presumably they arrive very early/first in the frame?
    mut cursor_moved_events: EventReader<CursorMoved>,
    mut events: EventWriter<PickedEvent>,
    render_device: Res<RenderDevice>,
) {
    #[cfg(feature = "trace")]
    let _picking_span = info_span!("picking", name = "picking").entered();

    for (picking, camera) in query.iter() {
        let Some(camera_size) = camera.physical_target_size() else { continue };

        if camera_size.x == 0 || camera_size.y == 0 {
            continue;
        }

        // TODO: Is it possible the GPU tries this at the same time as us?
        let guard = picking.buffer.try_lock().unwrap();

        let Some((buffer, picking_buffer_size)) = guard.as_ref() else { continue };

        let buffer_slice = buffer.slice(..);

        buffer_slice.map_async(MapMode::Read, move |result| {
            if let Err(e) = result {
                panic!("{e}");
            }
        });
        // For the above mapping to complete
        render_device.poll(Maintain::Wait);

        let buffer_view = buffer_slice.get_mapped_range();

        for event in cursor_moved_events.iter() {
            let picked_index = cursor_coords_to_entity_index(
                event.position,
                camera_size,
                picking_buffer_size,
                &buffer_view,
            );

            match *picked {
                Some(cached_index) if picked_index == u32::MAX => {
                    // No entity
                    events.send(PickedEvent::new_unpicked(cached_index));
                    *picked = None;
                }
                Some(cached_index) if cached_index == picked_index => {
                    // Nothing to report, the same entity is being hovered/picked
                }
                Some(cached_index) => {
                    // The cursor moved straight between two entities
                    events.send(PickedEvent::new_unpicked(cached_index));

                    *picked = Some(picked_index);
                    events.send(PickedEvent::new_picked(picked_index));
                }
                None if picked_index == u32::MAX => {
                    // Nothing to report, this index is reserved to mean "nothing picked"
                }
                None => {
                    *picked = Some(picked_index);
                    events.send(PickedEvent::new_picked(picked_index));
                }
            }
        }

        drop(buffer_view);
        buffer.unmap();
    }
}

pub fn prepare_picking_targets(
    mut commands: Commands,
    msaa: Res<Msaa>,
    render_device: Res<RenderDevice>,
    mut texture_cache: ResMut<TextureCache>,
    cameras: Query<(Entity, &ExtractedCamera, &Picking)>,
) {
    #[cfg(feature = "trace")]
    let _picking_span = info_span!("picking_prepare", name = "picking_prepare").entered();

    let mut textures = HashMap::default();
    for (entity, camera, picking) in cameras.iter() {
        if let Some(target_size) = camera.physical_target_size {
            let size = Extent3d {
                width: target_size.x,
                height: target_size.y,
                depth_or_array_layers: 1,
            };
            let picking_buffer_dimensions = PickingBufferSize::from(size);
            let needed_buffer_size = picking_buffer_dimensions.total_needed_bytes();

            let mut buffer = picking
                .buffer
                .try_lock()
                .expect("TODO: Are we ok to lock here?");

            let make_buffer = || {
                #[cfg(feature = "trace")]
                bevy_utils::tracing::debug!("Creating new picking buffer");

                render_device.create_buffer(&BufferDescriptor {
                    label: Some("Picking buffer"),
                    size: needed_buffer_size,
                    usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
                    mapped_at_creation: false,
                })
            };

            // If either the buffer has never been created or
            // the size of the current one has changed (e.g. due to window resize)
            // we have to create a new one.
            match buffer.as_mut() {
                Some((buffer, contained_size)) => {
                    if buffer.size() != needed_buffer_size {
                        *buffer = make_buffer();
                        *contained_size = size.into();
                    }
                }
                None => *buffer = Some((make_buffer(), size.into())),
            }

            // We want to store entity indices, which are u32s.
            // We therefore only need a single u32 channel.
            let picking_texture_format = TextureFormat::R32Uint;

            let picking_textures = textures.entry(camera.target.clone()).or_insert_with(|| {
                let descriptor = TextureDescriptor {
                    label: None,
                    size,
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: TextureDimension::D2,
                    format: picking_texture_format,
                    usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC,
                };

                PickingTextures {
                    main: texture_cache.get(
                        &render_device,
                        TextureDescriptor {
                            label: Some("main_picking_texture"),
                            ..descriptor
                        },
                    ),
                    sampled: (msaa.samples > 1).then(|| {
                        texture_cache.get(
                            &render_device,
                            TextureDescriptor {
                                label: Some("main_picking_texture_sampled"),
                                size,
                                mip_level_count: 1,
                                sample_count: msaa.samples,
                                dimension: TextureDimension::D2,
                                format: picking_texture_format,
                                usage: TextureUsages::RENDER_ATTACHMENT,
                            },
                        )
                    }),
                }
            });

            commands.entity(entity).insert(picking_textures.clone());
        }
    }
}
