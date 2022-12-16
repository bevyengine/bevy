use crate::{
    prelude::Image,
    texture::{BevyDefault, DefaultImageSampler, GpuImage, ImageSampler},
};
use bevy_derive::Deref;
use bevy_ecs::{prelude::FromWorld, system::Resource};
use bevy_gpu::{gpu_resource::*, GpuDevice, GpuQueue};
use bevy_math::Vec2;

/// A [`RenderApp`](crate::RenderApp) resource that contains the default "fallback image",
/// which can be used in situations where an image was not explicitly defined. The most common
/// use case is [`AsBindGroup`](crate::as_bind_group::AsBindGroup) implementations (such as materials) that support optional textures.
/// [`FallbackImage`] defaults to a 1x1 fully white texture, making blending colors with it a no-op.
#[derive(Resource, Deref)]
pub struct FallbackImage(GpuImage);

impl FromWorld for FallbackImage {
    fn from_world(world: &mut bevy_ecs::prelude::World) -> Self {
        let gpu_device = world.resource::<GpuDevice>();
        let gpu_queue = world.resource::<GpuQueue>();
        let default_sampler = world.resource::<DefaultImageSampler>();
        let image = Image::new_fill(
            Extent3d::default(),
            TextureDimension::D2,
            &[255u8; 4],
            TextureFormat::bevy_default(),
        );
        let texture =
            gpu_device.create_texture_with_data(gpu_queue, &image.texture_descriptor, &image.data);
        let texture_view = texture.create_view(&TextureViewDescriptor::default());
        let sampler = match image.sampler_descriptor {
            ImageSampler::Default => (**default_sampler).clone(),
            ImageSampler::Descriptor(descriptor) => gpu_device.create_sampler(&descriptor),
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
