use crate::{render_resource::*, texture::DefaultImageSampler};
use bevy_derive::Deref;
use bevy_ecs::prelude::FromWorld;
use bevy_math::Vec2;
use wgpu::{Extent3d, TextureDimension, TextureFormat};

use crate::{
    prelude::Image,
    renderer::{RenderDevice, RenderQueue},
    texture::{BevyDefault, GpuImage, ImageSampler},
};

/// A [`RenderApp`](crate::RenderApp) resource that contains the default "fallback image",
/// which can be used in situations where an image was not explicitly defined. The most common
/// use case is [`AsBindGroup`] implementations (such as materials) that support optional textures.
/// [`FallbackImage`] defaults to a 1x1 fully white texture, making blending colors with it a no-op.
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
        let texture = render_device.create_texture_with_data(
            render_queue,
            &image.texture_descriptor,
            &image.data,
        );
        let texture_view = texture.create_view(&TextureViewDescriptor::default());
        let sampler = match image.sampler_descriptor {
            ImageSampler::Default => (**default_sampler).clone(),
            ImageSampler::Descriptor(descriptor) => render_device.create_sampler(&descriptor),
        };
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
