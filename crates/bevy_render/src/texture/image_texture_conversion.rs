use crate::{
    render_asset::RenderAssetUsages,
    texture::{Image, TextureFormatPixelInfo},
};
use image::{DynamicImage, ImageBuffer};
use thiserror::Error;
use wgpu::{Extent3d, TextureDimension, TextureFormat};

impl Image {
    /// Converts a [`DynamicImage`] to an [`Image`].
    pub fn from_dynamic(
        dyn_img: DynamicImage,
        is_srgb: bool,
        asset_usage: RenderAssetUsages,
    ) -> Image {
        use bytemuck::cast_slice;
        let width;
        let height;

        let data: Vec<u8>;
        let format: TextureFormat;

        match dyn_img {
            DynamicImage::ImageLuma8(image) => {
                let i = DynamicImage::ImageLuma8(image).into_rgba8();
                width = i.width();
                height = i.height();
                format = if is_srgb {
                    TextureFormat::Rgba8UnormSrgb
                } else {
                    TextureFormat::Rgba8Unorm
                };

                data = i.into_raw();
            }
            DynamicImage::ImageLumaA8(image) => {
                let i = DynamicImage::ImageLumaA8(image).into_rgba8();
                width = i.width();
                height = i.height();
                format = if is_srgb {
                    TextureFormat::Rgba8UnormSrgb
                } else {
                    TextureFormat::Rgba8Unorm
                };

                data = i.into_raw();
            }
            DynamicImage::ImageRgb8(image) => {
                let i = DynamicImage::ImageRgb8(image).into_rgba8();
                width = i.width();
                height = i.height();
                format = if is_srgb {
                    TextureFormat::Rgba8UnormSrgb
                } else {
                    TextureFormat::Rgba8Unorm
                };

                data = i.into_raw();
            }
            DynamicImage::ImageRgba8(image) => {
                width = image.width();
                height = image.height();
                format = if is_srgb {
                    TextureFormat::Rgba8UnormSrgb
                } else {
                    TextureFormat::Rgba8Unorm
                };

                data = image.into_raw();
            }
            DynamicImage::ImageLuma16(image) => {
                width = image.width();
                height = image.height();
                format = TextureFormat::R16Uint;

                let raw_data = image.into_raw();

                data = cast_slice(&raw_data).to_owned();
            }
            DynamicImage::ImageLumaA16(image) => {
                width = image.width();
                height = image.height();
                format = TextureFormat::Rg16Uint;

                let raw_data = image.into_raw();

                data = cast_slice(&raw_data).to_owned();
            }
            DynamicImage::ImageRgb16(image) => {
                let i = DynamicImage::ImageRgb16(image).into_rgba16();
                width = i.width();
                height = i.height();
                format = TextureFormat::Rgba16Unorm;

                let raw_data = i.into_raw();

                data = cast_slice(&raw_data).to_owned();
            }
            DynamicImage::ImageRgba16(image) => {
                width = image.width();
                height = image.height();
                format = TextureFormat::Rgba16Unorm;

                let raw_data = image.into_raw();

                data = cast_slice(&raw_data).to_owned();
            }
            DynamicImage::ImageRgb32F(image) => {
                width = image.width();
                height = image.height();
                format = TextureFormat::Rgba32Float;

                let mut local_data =
                    Vec::with_capacity(width as usize * height as usize * format.pixel_size());

                for pixel in image.into_raw().chunks_exact(3) {
                    // TODO: use the array_chunks method once stabilised
                    // https://github.com/rust-lang/rust/issues/74985
                    let r = pixel[0];
                    let g = pixel[1];
                    let b = pixel[2];
                    let a = 1f32;

                    local_data.extend_from_slice(&r.to_ne_bytes());
                    local_data.extend_from_slice(&g.to_ne_bytes());
                    local_data.extend_from_slice(&b.to_ne_bytes());
                    local_data.extend_from_slice(&a.to_ne_bytes());
                }

                data = local_data;
            }
            DynamicImage::ImageRgba32F(image) => {
                width = image.width();
                height = image.height();
                format = TextureFormat::Rgba32Float;

                let raw_data = image.into_raw();

                data = cast_slice(&raw_data).to_owned();
            }
            // DynamicImage is now non exhaustive, catch future variants and convert them
            _ => {
                let image = dyn_img.into_rgba8();
                width = image.width();
                height = image.height();
                format = TextureFormat::Rgba8UnormSrgb;

                data = image.into_raw();
            }
        }

        Image::new(
            Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            data,
            format,
            asset_usage,
        )
    }

    /// Convert a [`Image`] to a [`DynamicImage`]. Useful for editing image
    /// data. Not all [`TextureFormat`] are covered, therefore it will return an
    /// error if the format is unsupported. Supported formats are:
    /// - `TextureFormat::R8Unorm`
    /// - `TextureFormat::Rg8Unorm`
    /// - `TextureFormat::Rgba8UnormSrgb`
    /// - `TextureFormat::Bgra8UnormSrgb`
    ///
    /// To convert [`Image`] to a different format see: [`Image::convert`].
    pub fn try_into_dynamic(self) -> Result<DynamicImage, IntoDynamicImageError> {
        match self.texture_descriptor.format {
            TextureFormat::R8Unorm => ImageBuffer::from_raw(self.width(), self.height(), self.data)
                .map(DynamicImage::ImageLuma8),
            TextureFormat::Rg8Unorm => {
                ImageBuffer::from_raw(self.width(), self.height(), self.data)
                    .map(DynamicImage::ImageLumaA8)
            }
            TextureFormat::Rgba8UnormSrgb => {
                ImageBuffer::from_raw(self.width(), self.height(), self.data)
                    .map(DynamicImage::ImageRgba8)
            }
            // This format is commonly used as the format for the swapchain texture
            // This conversion is added here to support screenshots
            TextureFormat::Bgra8UnormSrgb | TextureFormat::Bgra8Unorm => {
                ImageBuffer::from_raw(self.width(), self.height(), {
                    let mut data = self.data;
                    for bgra in data.chunks_exact_mut(4) {
                        bgra.swap(0, 2);
                    }
                    data
                })
                .map(DynamicImage::ImageRgba8)
            }
            // Throw and error if conversion isn't supported
            texture_format => return Err(IntoDynamicImageError::UnsupportedFormat(texture_format)),
        }
        .ok_or(IntoDynamicImageError::UnknownConversionError(
            self.texture_descriptor.format,
        ))
    }
}

/// Errors that occur while converting an [`Image`] into a [`DynamicImage`]
#[non_exhaustive]
#[derive(Error, Debug)]
pub enum IntoDynamicImageError {
    /// Conversion into dynamic image not supported for source format.
    #[error("Conversion into dynamic image not supported for {0:?}.")]
    UnsupportedFormat(TextureFormat),

    /// Encountered an unknown error during conversion.
    #[error("Failed to convert into {0:?}.")]
    UnknownConversionError(TextureFormat),
}

#[cfg(test)]
mod test {
    use image::{GenericImage, Rgba};

    use super::*;

    #[test]
    fn two_way_conversion() {
        // Check to see if color is preserved through an rgba8 conversion and back.
        let mut initial = DynamicImage::new_rgba8(1, 1);
        initial.put_pixel(0, 0, Rgba::from([132, 3, 7, 200]));

        let image = Image::from_dynamic(initial.clone(), true, RenderAssetUsages::RENDER_WORLD);

        // NOTE: Fails if `is_srbg = false` or the dynamic image is of the type rgb8.
        assert_eq!(initial, image.try_into_dynamic().unwrap());
    }
}
