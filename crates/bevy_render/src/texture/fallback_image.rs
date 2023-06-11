use crate::{render_resource::*, texture::DefaultImageSampler};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    prelude::{FromWorld, Res, ResMut},
    system::{Resource, SystemParam},
};
use bevy_math::Vec2;
use bevy_utils::HashMap;
use wgpu::{Extent3d, TextureFormat};

use crate::{
    prelude::Image,
    renderer::{RenderDevice, RenderQueue},
    texture::{image::TextureFormatPixelInfo, BevyDefault, GpuImage, ImageSampler},
};

/// A [`RenderApp`](crate::RenderApp) resource that contains the default "fallback image",
/// which can be used in situations where an image was not explicitly defined. The most common
/// use case is [`AsBindGroup`] implementations (such as materials) that support optional textures.
///
/// Defaults to a 1x1 fully opaque white texture, (1.0, 1.0, 1.0, 1.0) which makes multiplying
/// it with other colors a no-op.
#[derive(Resource)]
pub struct FallbackImage {
    /// Fallback image for [`TextureViewDimension::D1`].
    pub d1: GpuImage,
    /// Fallback image for [`TextureViewDimension::D2`].
    pub d2: GpuImage,
    /// Fallback image for [`TextureViewDimension::D2Array`].
    pub d2_array: GpuImage,
    /// Fallback image for [`TextureViewDimension::Cube`].
    pub cube: GpuImage,
    /// Fallback image for [`TextureViewDimension::CubeArray`].
    pub cube_array: GpuImage,
    /// Fallback image for [`TextureViewDimension::D3`].
    pub d3: GpuImage,
}

/// A [`RenderApp`](crate::RenderApp) resource that contains a _zero-filled_ "fallback image",
/// which can be used in place of [`FallbackImage`], when a fully transparent or black fallback
/// is required instead of fully opaque white.
///
/// Defaults to a 1x1 fully transparent black texture, (0.0, 0.0, 0.0, 0.0) which makes adding
/// or alpha-blending it to other colors a no-op.
#[derive(Resource, Deref)]
pub struct FallbackImageZero(GpuImage);

/// A [`RenderApp`](crate::RenderApp) resource that contains a "cubemap fallback image",
/// which can be used in situations where an image was not explicitly defined. The most common
/// use case is [`AsBindGroup`] implementations (such as materials) that support optional textures.
#[derive(Resource, Deref)]
pub struct FallbackImageCubemap(GpuImage);

fn fallback_image_new(
    render_device: &RenderDevice,
    render_queue: &RenderQueue,
    default_sampler: &DefaultImageSampler,
    format: TextureFormat,
    dimension: TextureViewDimension,
    samples: u32,
    value: u8,
) -> GpuImage {
    // TODO make this configurable per channel
    let data = vec![value; format.pixel_size()];

    let extents = Extent3d {
        width: 1,
        height: 1,
        depth_or_array_layers: match dimension {
            TextureViewDimension::Cube | TextureViewDimension::CubeArray => 6,
            _ => 1,
        },
    };

    let image_dimension = dimension.compatible_texture_dimension();

    let mut image = Image::new_fill(extents, image_dimension, &data, format);
    image.texture_descriptor.sample_count = samples;
    if image_dimension == TextureDimension::D2 {
        image.texture_descriptor.usage |= TextureUsages::RENDER_ATTACHMENT;
    }

    // We can't create textures with data when it's a depth texture or when using multiple samples
    let texture = if format.is_depth_stencil_format() || samples > 1 {
        render_device.create_texture(&image.texture_descriptor)
    } else {
        render_device.create_texture_with_data(render_queue, &image.texture_descriptor, &image.data)
    };

    let texture_view = texture.create_view(&TextureViewDescriptor {
        dimension: Some(dimension),
        array_layer_count: Some(extents.depth_or_array_layers),
        ..TextureViewDescriptor::default()
    });
    let sampler = match image.sampler_descriptor {
        ImageSampler::Default => (**default_sampler).clone(),
        ImageSampler::Descriptor(descriptor) => render_device.create_sampler(&descriptor),
    };
    GpuImage {
        texture,
        texture_view,
        texture_format: image.texture_descriptor.format,
        sampler,
        size: Vec2::new(
            image.texture_descriptor.size.width as f32,
            image.texture_descriptor.size.height as f32,
        ),
        mip_level_count: image.texture_descriptor.mip_level_count,
    }
}

impl FromWorld for FallbackImage {
    fn from_world(world: &mut bevy_ecs::prelude::World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let render_queue = world.resource::<RenderQueue>();
        let default_sampler = world.resource::<DefaultImageSampler>();
        Self {
            d1: fallback_image_new(
                render_device,
                render_queue,
                default_sampler,
                TextureFormat::bevy_default(),
                TextureViewDimension::D1,
                1,
                255,
            ),
            d2: fallback_image_new(
                render_device,
                render_queue,
                default_sampler,
                TextureFormat::bevy_default(),
                TextureViewDimension::D2,
                1,
                255,
            ),
            d2_array: fallback_image_new(
                render_device,
                render_queue,
                default_sampler,
                TextureFormat::bevy_default(),
                TextureViewDimension::D2Array,
                1,
                255,
            ),
            cube: fallback_image_new(
                render_device,
                render_queue,
                default_sampler,
                TextureFormat::bevy_default(),
                TextureViewDimension::Cube,
                1,
                255,
            ),
            cube_array: fallback_image_new(
                render_device,
                render_queue,
                default_sampler,
                TextureFormat::bevy_default(),
                TextureViewDimension::CubeArray,
                1,
                255,
            ),
            d3: fallback_image_new(
                render_device,
                render_queue,
                default_sampler,
                TextureFormat::bevy_default(),
                TextureViewDimension::D3,
                1,
                255,
            ),
        }
    }
}

impl FromWorld for FallbackImageZero {
    fn from_world(world: &mut bevy_ecs::prelude::World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let render_queue = world.resource::<RenderQueue>();
        let default_sampler = world.resource::<DefaultImageSampler>();
        Self(fallback_image_new(
            render_device,
            render_queue,
            default_sampler,
            TextureFormat::bevy_default(),
            TextureViewDimension::D2,
            1,
            0,
        ))
    }
}

impl FromWorld for FallbackImageCubemap {
    fn from_world(world: &mut bevy_ecs::prelude::World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let render_queue = world.resource::<RenderQueue>();
        let default_sampler = world.resource::<DefaultImageSampler>();
        Self(fallback_image_new(
            render_device,
            render_queue,
            default_sampler,
            TextureFormat::bevy_default(),
            TextureViewDimension::Cube,
            1,
            255,
        ))
    }
}

// TODO these could be combined in one FallbackImage cache.

/// A Cache of fallback textures that uses the sample count as a key
///
/// # WARNING
/// Images using MSAA with sample count > 1 are not initialized with data, therefore,
/// you shouldn't sample them before writing data to them first.
#[derive(Resource, Deref, DerefMut, Default)]
pub struct FallbackImageMsaaCache(HashMap<u32, GpuImage>);

/// A Cache of fallback depth textures that uses the sample count as a key
///
/// # WARNING
/// Depth images are never initialized with data, therefore,
/// you shouldn't sample them before writing data to them first.
#[derive(Resource, Deref, DerefMut, Default)]
pub struct FallbackImageDepthCache(HashMap<u32, GpuImage>);

#[derive(SystemParam)]
pub struct FallbackImagesMsaa<'w> {
    cache: ResMut<'w, FallbackImageMsaaCache>,
    render_device: Res<'w, RenderDevice>,
    render_queue: Res<'w, RenderQueue>,
    default_sampler: Res<'w, DefaultImageSampler>,
}

impl<'w> FallbackImagesMsaa<'w> {
    pub fn image_for_samplecount(&mut self, sample_count: u32) -> &GpuImage {
        self.cache.entry(sample_count).or_insert_with(|| {
            fallback_image_new(
                &self.render_device,
                &self.render_queue,
                &self.default_sampler,
                TextureFormat::bevy_default(),
                TextureViewDimension::D2,
                sample_count,
                255,
            )
        })
    }
}

#[derive(SystemParam)]
pub struct FallbackImagesDepth<'w> {
    cache: ResMut<'w, FallbackImageDepthCache>,
    render_device: Res<'w, RenderDevice>,
    render_queue: Res<'w, RenderQueue>,
    default_sampler: Res<'w, DefaultImageSampler>,
}

impl<'w> FallbackImagesDepth<'w> {
    pub fn image_for_samplecount(&mut self, sample_count: u32) -> &GpuImage {
        self.cache.entry(sample_count).or_insert_with(|| {
            fallback_image_new(
                &self.render_device,
                &self.render_queue,
                &self.default_sampler,
                TextureFormat::Depth32Float,
                TextureViewDimension::D2,
                sample_count,
                255,
            )
        })
    }
}
