#[cfg(feature = "hdr")]
mod hdr_texture_loader;
mod image_texture_loader;
#[allow(clippy::module_inception)]
mod texture;
mod texture_cache;

pub(crate) mod image_texture_conversion;

#[cfg(feature = "hdr")]
pub use hdr_texture_loader::*;
pub use image_texture_loader::*;
pub use texture::*;
pub use texture_cache::*;

use crate::{
    renderer::{RenderDevice, RenderQueue},
    RenderStage,
};
use bevy_app::{App, CoreStage, Plugin};
use bevy_asset::{AddAsset, AssetEvent, Assets};
use bevy_ecs::prelude::*;
use bevy_utils::HashSet;
use wgpu::{ImageCopyTexture, ImageDataLayout, Origin3d, TextureViewDescriptor};

// TODO: replace Texture names with Image names?
pub struct ImagePlugin;

impl Plugin for ImagePlugin {
    fn build(&self, app: &mut App) {
        #[cfg(feature = "png")]
        {
            app.init_asset_loader::<ImageTextureLoader>();
        }

        app.add_system_to_stage(CoreStage::PostUpdate, image_resource_system.system())
            .add_asset::<Image>();

        let render_app = app.sub_app_mut(0);
        render_app
            .init_resource::<TextureCache>()
            .add_system_to_stage(RenderStage::Cleanup, update_texture_cache_system.system());
    }
}

pub fn image_resource_system(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut images: ResMut<Assets<Image>>,
    mut image_events: EventReader<AssetEvent<Image>>,
) {
    let mut changed_images = HashSet::default();
    for event in image_events.iter() {
        match event {
            AssetEvent::Created { handle } => {
                changed_images.insert(handle);
            }
            AssetEvent::Modified { handle } => {
                changed_images.insert(handle);
                // TODO: uncomment this to support mutated textures
                // remove_current_texture_resources(render_resource_context, handle, &mut textures);
            }
            AssetEvent::Removed { handle } => {
                // if texture was modified and removed in the same update, ignore the
                // modification events are ordered so future modification
                // events are ok
                changed_images.remove(handle);
            }
        }
    }

    for image_handle in changed_images.iter() {
        if let Some(image) = images.get_mut(*image_handle) {
            // TODO: this avoids creating new textures each frame because storing gpu data in the texture flags it as
            // modified. this prevents hot reloading and therefore can't be used in an actual impl.
            if image.gpu_data.is_some() {
                continue;
            }

            let texture = render_device.create_texture(&image.texture_descriptor);
            let sampler = render_device.create_sampler(&image.sampler_descriptor);

            let width = image.texture_descriptor.size.width as usize;
            let format_size = image.texture_descriptor.format.pixel_size();
            // let mut aligned_data = vec![
            //     0;
            //     format_size
            //         * aligned_width
            //         * image.texture_descriptor.size.height as usize
            //         * image.texture_descriptor.size.depth_or_array_layers
            //             as usize
            // ];
            // image
            //     .data
            //     .chunks_exact(format_size * width)
            //     .enumerate()
            //     .for_each(|(index, row)| {
            //         let offset = index * aligned_width * format_size;
            //         aligned_data[offset..(offset + width * format_size)].copy_from_slice(row);
            //     });

            // TODO: this might require different alignment. docs seem to say that we don't need it though
            render_queue.write_texture(
                ImageCopyTexture {
                    texture: &texture,
                    mip_level: 0,
                    origin: Origin3d::ZERO,
                },
                &image.data,
                ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(
                        std::num::NonZeroU32::new(
                            image.texture_descriptor.size.width * format_size as u32,
                        )
                        .unwrap(),
                    ),
                    rows_per_image: None,
                },
                image.texture_descriptor.size,
            );

            let texture_view = texture.create_view(&TextureViewDescriptor::default());
            image.gpu_data = Some(ImageGpuData {
                texture,
                texture_view,
                sampler,
            });
        }
    }
}

pub trait BevyDefault {
    fn bevy_default() -> Self;
}

impl BevyDefault for wgpu::TextureFormat {
    fn bevy_default() -> Self {
        if cfg!(target_os = "android") {
            // Bgra8UnormSrgb texture missing on some Android devices
            wgpu::TextureFormat::Rgba8UnormSrgb
        } else {
            wgpu::TextureFormat::Bgra8UnormSrgb
        }
    }
}
