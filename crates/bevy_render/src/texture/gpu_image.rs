use crate::{
    render_asset::{AssetExtractionError, PrepareAssetError, RenderAsset},
    render_resource::{DefaultImageSampler, Sampler, Texture, TextureView},
    renderer::{RenderDevice, RenderQueue},
};
use bevy_asset::{AssetId, RenderAssetUsages};
use bevy_ecs::system::{lifetimeless::SRes, SystemParamItem};
use bevy_image::{Image, ImageSampler};
use bevy_math::{AspectRatio, UVec2};
use tracing::warn;
use wgpu::{Extent3d, TextureFormat, TextureViewDescriptor};

/// The GPU-representation of an [`Image`].
/// Consists of the [`Texture`], its [`TextureView`] and the corresponding [`Sampler`], and the texture's size.
#[derive(Debug, Clone)]
pub struct GpuImage {
    pub texture: Texture,
    pub texture_view: TextureView,
    pub texture_format: TextureFormat,
    pub texture_view_format: Option<TextureFormat>,
    pub sampler: Sampler,
    pub size: Extent3d,
    pub mip_level_count: u32,
    pub had_data: bool,
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

    fn take_gpu_data(
        source: &mut Self::SourceAsset,
        previous_gpu_asset: Option<&Self>,
    ) -> Result<Self::SourceAsset, AssetExtractionError> {
        let data = source.data.take();

        // check if this image originally had data and no longer does, that implies it
        // has already been extracted
        let valid_upload = data.is_some() || previous_gpu_asset.is_none_or(|prev| !prev.had_data);

        valid_upload
            .then(|| Self::SourceAsset {
                data,
                ..source.clone()
            })
            .ok_or(AssetExtractionError::AlreadyExtracted)
    }

    #[inline]
    fn byte_len(image: &Self::SourceAsset) -> Option<usize> {
        image.data.as_ref().map(Vec::len)
    }

    /// Converts the extracted image into a [`GpuImage`].
    fn prepare_asset(
        image: Self::SourceAsset,
        _: AssetId<Self::SourceAsset>,
        (render_device, render_queue, default_sampler): &mut SystemParamItem<Self::Param>,
        previous_asset: Option<&Self>,
    ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        let had_data = image.data.is_some();
        let texture = if let Some(ref data) = image.data {
            render_device.create_texture_with_data(
                render_queue,
                &image.texture_descriptor,
                image.data_order,
                data,
            )
        } else {
            let new_texture = render_device.create_texture(&image.texture_descriptor);
            if image.copy_on_resize {
                if let Some(previous) = previous_asset {
                    let mut command_encoder =
                        render_device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                            label: Some("copy_image_on_resize"),
                        });
                    let copy_size = Extent3d {
                        width: image.texture_descriptor.size.width.min(previous.size.width),
                        height: image
                            .texture_descriptor
                            .size
                            .height
                            .min(previous.size.height),
                        depth_or_array_layers: image
                            .texture_descriptor
                            .size
                            .depth_or_array_layers
                            .min(previous.size.depth_or_array_layers),
                    };

                    command_encoder.copy_texture_to_texture(
                        previous.texture.as_image_copy(),
                        new_texture.as_image_copy(),
                        copy_size,
                    );
                    render_queue.submit([command_encoder.finish()]);
                } else {
                    warn!("No previous asset to copy from for image: {:?}", image);
                }
            }
            new_texture
        };

        let texture_view = texture.create_view(
            image
                .texture_view_descriptor
                .as_ref()
                .unwrap_or(&TextureViewDescriptor::default()),
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
            texture_view_format: image.texture_view_descriptor.and_then(|v| v.format),
            sampler,
            size: image.texture_descriptor.size,
            mip_level_count: image.texture_descriptor.mip_level_count,
            had_data,
        })
    }
}

impl GpuImage {
    /// Returns the aspect ratio (width / height) of a 2D image.
    #[inline]
    pub fn aspect_ratio(&self) -> AspectRatio {
        AspectRatio::try_from_pixels(self.size.width, self.size.height).expect(
            "Failed to calculate aspect ratio: Image dimensions must be positive, non-zero values",
        )
    }

    /// Returns the size of a 2D image.
    #[inline]
    pub fn size_2d(&self) -> UVec2 {
        UVec2::new(self.size.width, self.size.height)
    }
}
