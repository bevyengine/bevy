use crate::{
    render_asset::{AssetExtractionError, PrepareAssetError, RenderAsset},
    render_resource::{DefaultImageSampler, Sampler, Texture, TextureView},
    renderer::{RenderDevice, RenderQueue},
};
use bevy_asset::{AssetId, RenderAssetTransferPriority, RenderAssetUsages};
use bevy_ecs::system::{lifetimeless::SRes, SystemParamItem};
use bevy_image::{Image, ImageSampler};
use bevy_math::{AspectRatio, UVec2};
use tracing::warn;
use wgpu::{Extent3d, TexelCopyBufferLayout, TextureFormat, TextureUsages};
use wgpu_types::{TextureDescriptor, TextureViewDescriptor};

/// The GPU-representation of an [`Image`].
/// Consists of the [`Texture`], its [`TextureView`] and the corresponding [`Sampler`], and the texture's size.
#[derive(Debug, Clone)]
pub struct GpuImage {
    pub texture: Texture,
    pub texture_view: TextureView,
    pub sampler: Sampler,
    pub texture_descriptor: TextureDescriptor<Option<&'static str>, &'static [TextureFormat]>,
    pub texture_view_descriptor: Option<TextureViewDescriptor<Option<&'static str>>>,
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
    fn transfer_priority(
        image: &Self::SourceAsset,
    ) -> (RenderAssetTransferPriority, Option<usize>) {
        (image.transfer_priority, image.data.as_ref().map(Vec::len))
    }

    /// Converts the extracted image into a [`GpuImage`].
    fn prepare_asset(
        image: Self::SourceAsset,
        _: AssetId<Self::SourceAsset>,
        (render_device, render_queue, default_sampler): &mut SystemParamItem<Self::Param>,
        previous_asset: Option<&Self>,
    ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        let had_data = image.data.is_some();
        let texture = if let Some(prev) = previous_asset
            && prev.texture_descriptor == image.texture_descriptor
            && prev
                .texture_descriptor
                .usage
                .contains(TextureUsages::COPY_DST)
            && let Some(block_bytes) = image.texture_descriptor.format.block_copy_size(None)
        {
            if let Some(ref data) = image.data {
                let (block_width, block_height) =
                    image.texture_descriptor.format.block_dimensions();

                // queue copy
                render_queue.write_texture(
                    prev.texture.as_image_copy(),
                    data,
                    TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(image.width() / block_width * block_bytes),
                        rows_per_image: Some(image.height() / block_height),
                    },
                    image.texture_descriptor.size,
                );
            }

            if !image.copy_on_resize {
                // TODO else could clear here? probably not necessary as textures without data are only useful as render
                // targets and will normally be overwritten immediately anyway
            }

            // reuse previous texture
            prev.texture.clone()
        } else if let Some(ref data) = image.data {
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
                        width: image
                            .texture_descriptor
                            .size
                            .width
                            .min(previous.texture_descriptor.size.width),
                        height: image
                            .texture_descriptor
                            .size
                            .height
                            .min(previous.texture_descriptor.size.height),
                        depth_or_array_layers: image
                            .texture_descriptor
                            .size
                            .depth_or_array_layers
                            .min(previous.texture_descriptor.size.depth_or_array_layers),
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

        let texture_view = if let Some(prev) = previous_asset.as_ref()
            && prev.texture_descriptor == image.texture_descriptor
            && prev
                .texture_descriptor
                .usage
                .contains(TextureUsages::COPY_DST)
            && prev.texture_view_descriptor == image.texture_view_descriptor
        {
            prev.texture_view.clone()
        } else {
            image
                .texture_view_descriptor
                .as_ref()
                .map(|desc| texture.create_view(desc))
                .unwrap_or_else(|| texture.create_view(&TextureViewDescriptor::default()))
        };
        let sampler = match image.sampler {
            ImageSampler::Default => (***default_sampler).clone(),
            ImageSampler::Descriptor(descriptor) => {
                render_device.create_sampler(&descriptor.as_wgpu())
            }
        };

        Ok(GpuImage {
            texture,
            texture_view,
            sampler,
            texture_descriptor: image.texture_descriptor,
            texture_view_descriptor: image.texture_view_descriptor,
            had_data,
        })
    }
}

impl GpuImage {
    /// Returns the aspect ratio (width / height) of a 2D image.
    #[inline]
    pub fn aspect_ratio(&self) -> AspectRatio {
        AspectRatio::try_from_pixels(
            self.texture_descriptor.size.width,
            self.texture_descriptor.size.height,
        )
        .expect(
            "Failed to calculate aspect ratio: Image dimensions must be positive, non-zero values",
        )
    }

    /// Returns the size of a 2D image.
    #[inline]
    pub fn size_2d(&self) -> UVec2 {
        UVec2::new(
            self.texture_descriptor.size.width,
            self.texture_descriptor.size.height,
        )
    }

    /// Gets the view format of this image.
    /// If the view format is not explicitly provided, falls back to the base image format
    #[inline]
    pub fn view_format(&self) -> TextureFormat {
        self.texture_view_descriptor
            .as_ref()
            .and_then(|view_desc| view_desc.format)
            .unwrap_or(self.texture_descriptor.format)
    }
}
