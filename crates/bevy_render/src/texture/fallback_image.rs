use crate::{
    render_resource::*,
    texture::{DefaultImageSampler, TextureFormatPixelInfo},
};
use bevy_derive::Deref;
use bevy_ecs::prelude::FromWorld;
use bevy_math::Vec2;
use wgpu::{Extent3d, ImageCopyTexture, Origin3d, TextureDimension, TextureFormat};

use crate::{
    prelude::Image,
    renderer::{RenderDevice, RenderQueue},
    texture::{BevyDefault, GpuImage, ImageSampler},
};

#[derive(Deref)]
pub struct FallbackImage(GpuImage);

impl FromWorld for FallbackImage {
    fn from_world(world: &mut bevy_ecs::prelude::World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let render_queue = world.resource::<RenderQueue>();
        let default_sampler = world.resource::<DefaultImageSampler>();
        let image = Image::new_fill(
            Extent3d::default(),
            TextureDimension::D2,
            &[255u8; 4],
            TextureFormat::bevy_default(),
        );
        let texture = render_device.create_texture(&image.texture_descriptor);
        let sampler = match image.sampler_descriptor {
            ImageSampler::Default => (**default_sampler).clone(),
            ImageSampler::Descriptor(descriptor) => render_device.create_sampler(&descriptor),
        };

        let format_size = image.texture_descriptor.format.pixel_size();
        render_queue.write_texture(
            ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: Origin3d::ZERO,
                aspect: TextureAspect::All,
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
        Self(GpuImage {
            texture,
            texture_view,
            texture_format: image.texture_descriptor.format,
            sampler,
            size: Vec2::new(
                image.texture_descriptor.size.width as f32,
                image.texture_descriptor.size.height as f32,
            ),
        })
    }
}
