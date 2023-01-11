use std::{num::NonZeroU32, path::Path};

use bevy_ecs::prelude::*;
use bevy_log::info_span;
use bevy_tasks::AsyncComputeTaskPool;
use bevy_utils::HashMap;
use bevy_window::WindowId;
use parking_lot::Mutex;
use thiserror::Error;
use wgpu::{
    CommandEncoder, Extent3d, ImageDataLayout, TextureFormat, COPY_BYTES_PER_ROW_ALIGNMENT,
};

use crate::{prelude::Image, texture::TextureFormatPixelInfo};

use super::ExtractedWindows;

pub type ScreenshotFn = Box<dyn FnOnce(Image) + Send + Sync>;

/// A resource which allows for taking screenshots of the window.
#[derive(Resource, Default)]
pub struct ScreenshotManager {
    // this is in a mutex to enable extraction with only an immutable reference
    pub(crate) callbacks: Mutex<HashMap<WindowId, ScreenshotFn>>,
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
        window: WindowId,
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
        window: WindowId,
        path: impl AsRef<Path>,
    ) -> Result<(), ScreenshotAlreadyRequestedError> {
        let path = path.as_ref().to_owned();
        self.take_screenshot(window, |image| {
            image.try_into_dynamic().unwrap().save(path).unwrap();
        })
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
            NonZeroU32::new(get_aligned_size(width, 1, format.pixel_size() as u32))
        } else {
            None
        },
        rows_per_image: None,
        ..Default::default()
    }
}

pub(crate) fn submit_screenshot_commands(windows: &ExtractedWindows, encoder: &mut CommandEncoder) {
    for (window, texture) in windows
        .values()
        .filter_map(|w| w.swap_chain_texture.as_ref().map(|t| (w, t)))
    {
        if let Some(screenshot_buffer) = &window.screenshot_buffer {
            let width = window.physical_width;
            let height = window.physical_height;
            let texture_format = window.swap_chain_texture_format.unwrap();
            let texture = &texture.get_surface_texture().unwrap().texture;

            encoder.copy_texture_to_buffer(
                texture.as_image_copy(),
                wgpu::ImageCopyBuffer {
                    buffer: screenshot_buffer,
                    layout: crate::view::screenshot::layout_data(width, height, texture_format),
                },
                Extent3d {
                    width,
                    height,
                    ..Default::default()
                },
            );
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
            let buffer = window.screenshot_buffer.take().unwrap();

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
