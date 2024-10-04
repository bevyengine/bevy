use crate::{
    render_asset::{PrepareAssetError, RenderAsset, RenderAssetUsages},
    render_resource::{DefaultImageSampler, Sampler, Texture, TextureView},
    renderer::{RenderDevice, RenderQueue},
};
use bevy_ecs::system::{lifetimeless::SRes, SystemParamItem};
use bevy_image::{Image, ImageSampler};
use bevy_math::UVec2;
use wgpu::{TextureFormat, TextureViewDescriptor};

/// The GPU-representation of an [`Image`].
/// Consists of the [`Texture`], its [`TextureView`] and the corresponding [`Sampler`], and the texture's size.
#[derive(Debug, Clone)]
pub struct GpuImage {
    pub texture: Texture,
    pub texture_view: TextureView,
    pub texture_format: TextureFormat,
    pub sampler: Sampler,
    pub size: UVec2,
    pub mip_level_count: u32,
}

impl RenderAsset for GpuImage {
    type SourceAsset = Image;
    type Param = (
        SRes<RenderDevice>,
        SRes<RenderQueue>,
        SRes<DefaultImageSampler>,
    );

    #[inline]
    fn asset_usage(image: &Self::SourceAsset) -> RenderAssetUsages {
        image.asset_usage
    }

    #[inline]
    fn byte_len(image: &Self::SourceAsset) -> Option<usize> {
        Some(image.data.len())
    }

    /// Converts the extracted image into a [`GpuImage`].
    fn prepare_asset(
        image: Self::SourceAsset,
        (render_device, render_queue, default_sampler): &mut SystemParamItem<Self::Param>,
    ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        let texture = render_device.create_texture_with_data(
            render_queue,
            &image.texture_descriptor,
            // TODO: Is this correct? Do we need to use `MipMajor` if it's a ktx2 file?
            wgpu::util::TextureDataOrder::default(),
            &image.data,
        );

        let size = image.size();
        let texture_view = texture.create_view(
            image
                .texture_view_descriptor
                .or_else(|| Some(TextureViewDescriptor::default()))
                .as_ref()
                .unwrap(),
        );
        let sampler = match image.sampler {
            ImageSampler::Default => (***default_sampler).clone(),
            ImageSampler::Descriptor(descriptor) => {
                render_device.create_sampler(&descriptor.as_wgpu())
            }
        };

        Ok(GpuImage {
            texture,
            texture_view,
            texture_format: image.texture_descriptor.format,
            sampler,
            size,
            mip_level_count: image.texture_descriptor.mip_level_count,
        })
    }
}
