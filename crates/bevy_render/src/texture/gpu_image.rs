use crate::{
    render_asset::{PrepareAssetError, RenderAsset},
    render_resource::{DefaultImageSampler, Sampler, Texture, TextureView},
    renderer::{RenderDevice, RenderQueue},
};
use bevy_asset::{AssetId, RenderAssetUsages, RetainedAsset};
use bevy_ecs::system::{lifetimeless::SRes, SystemParamItem};
use bevy_image::{Image, ImageSampler};
use bevy_log::warn;
use bevy_math::{AspectRatio, UVec2};
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
}

/// The representation of an [`Image`] that is retained in [`RetainedAssets`] on the main world after extracting.
///
/// [`RetainedAssets`]: bevy_render::render_asset::RetainedAssets
#[derive(Debug, Clone, PartialEq)]
pub struct RetainedImage {
    /// For texture data with layers and mips, this field controls how wgpu interprets the buffer layout.
    ///
    /// Use [`TextureDataOrder::default()`] for all other cases.
    pub data_order: wgpu_types::TextureDataOrder,
    // TODO: this nesting makes accessing Image metadata verbose. Either flatten out descriptor or add accessors.
    /// Describes the data layout of the GPU texture.\
    /// For example, whether a texture contains 1D/2D/3D data, and what the format of the texture data is.
    ///
    /// ## Field Usage Notes
    /// - [`TextureDescriptor::label`] is used for caching purposes when not using `Asset<Image>`.\
    ///   If you use assets, the label is purely a debugging aid.
    /// - [`TextureDescriptor::view_formats`] is currently unused by Bevy.
    pub texture_descriptor: TextureDescriptor<Option<&'static str>, &'static [TextureFormat]>,
    /// The [`ImageSampler`] to use during rendering.
    pub sampler: ImageSampler,
    /// Describes how the GPU texture should be interpreted.\
    /// For example, 2D image data could be read as plain 2D, an array texture of layers of 2D with the same dimensions (and the number of layers in that case),
    /// a cube map, an array of cube maps, etc.
    ///
    /// ## Field Usage Notes
    /// - [`TextureViewDescriptor::label`] is used for caching purposes when not using `Asset<Image>`.\
    ///   If you use assets, the label is purely a debugging aid.
    pub texture_view_descriptor: Option<TextureViewDescriptor<Option<&'static str>>>,
    /// Where this image asset will be used. See [`RenderAssetUsages`] for more.
    pub asset_usage: RenderAssetUsages,
    /// Whether this image should be copied on the GPU when resized.
    pub copy_on_resize: bool,
}

impl RetainedAsset for RetainedImage {
    type SourceAsset = Image;
}

impl RenderAsset for GpuImage {
    type SourceAsset = Image;
    type RetainedAsset = RetainedImage;
    type Param = (
        SRes<RenderDevice>,
        SRes<RenderQueue>,
        SRes<DefaultImageSampler>,
    );

    #[inline]
    fn asset_usage(image: &Self::SourceAsset) -> RenderAssetUsages {
        image.asset_usage
    }

    fn retain_main_world_asset(source: &mut Self::SourceAsset) -> Self::RetainedAsset {
        RetainedImage {
            data_order: source.data_order,
            texture_descriptor: source.texture_descriptor.clone(),
            sampler: source.sampler.clone(),
            texture_view_descriptor: source.texture_view_descriptor.clone(),
            asset_usage: source.asset_usage,
            copy_on_resize: source.copy_on_resize,
        }
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
        let texture = if let Some(prev) = previous_asset
            && prev.texture_descriptor == image.texture_descriptor
            && (!had_data
                || prev
                    .texture_descriptor
                    .usage
                    .contains(TextureUsages::COPY_DST))
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
