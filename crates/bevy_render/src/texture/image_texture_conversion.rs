use crate::texture::{Image, TextureFormatPixelInfo};
use anyhow::anyhow;
use image::{DynamicImage, ImageBuffer};
use wgpu::{Extent3d, TextureDimension, TextureFormat};

impl Image {
    /// Converts a [`DynamicImage`] to an [`Image`].
    pub fn from_dynamic(dyn_img: DynamicImage, is_srgb: bool) -> Image {
        use bevy_core::cast_slice;
        let width;
        let height;

        let data: Vec<u8>;
        let format: TextureFormat;

        match dyn_img {
            DynamicImage::ImageLuma8(i) => {
                let i = DynamicImage::ImageLuma8(i).into_rgba8();
                width = i.width();
                height = i.height();
                format = if is_srgb {
                    TextureFormat::Rgba8UnormSrgb
                } else {
                    TextureFormat::Rgba8Unorm
                };

                data = i.into_raw();
            }
            DynamicImage::ImageLumaA8(i) => {
                let i = DynamicImage::ImageLumaA8(i).into_rgba8();
                width = i.width();
                height = i.height();
                format = if is_srgb {
                    TextureFormat::Rgba8UnormSrgb
                } else {
                    TextureFormat::Rgba8Unorm
                };

                data = i.into_raw();
            }
            DynamicImage::ImageRgb8(i) => {
                let i = DynamicImage::ImageRgb8(i).into_rgba8();
                width = i.width();
                height = i.height();
                format = if is_srgb {
                    TextureFormat::Rgba8UnormSrgb
                } else {
                    TextureFormat::Rgba8Unorm
                };

                data = i.into_raw();
            }
            DynamicImage::ImageRgba8(i) => {
                width = i.width();
                height = i.height();
                format = if is_srgb {
                    TextureFormat::Rgba8UnormSrgb
                } else {
                    TextureFormat::Rgba8Unorm
                };

                data = i.into_raw();
            }
            DynamicImage::ImageLuma16(i) => {
                width = i.width();
                height = i.height();
                format = TextureFormat::R16Uint;

                let raw_data = i.into_raw();

                data = cast_slice(&raw_data).to_owned();
            }
            DynamicImage::ImageLumaA16(i) => {
                width = i.width();
                height = i.height();
                format = TextureFormat::Rg16Uint;

                let raw_data = i.into_raw();

                data = cast_slice(&raw_data).to_owned();
            }
            DynamicImage::ImageRgb16(image) => {
                width = image.width();
                height = image.height();
                format = TextureFormat::Rgba16Uint;

                let mut local_data =
                    Vec::with_capacity(width as usize * height as usize * format.pixel_size());

                for pixel in image.into_raw().chunks_exact(3) {
                    // TODO: use the array_chunks method once stabilised
                    // https://github.com/rust-lang/rust/issues/74985
                    let r = pixel[0];
                    let g = pixel[1];
                    let b = pixel[2];
                    let a = u16::max_value();

                    local_data.extend_from_slice(&r.to_ne_bytes());
                    local_data.extend_from_slice(&g.to_ne_bytes());
                    local_data.extend_from_slice(&b.to_ne_bytes());
                    local_data.extend_from_slice(&a.to_ne_bytes());
                }

                data = local_data;
            }
            DynamicImage::ImageRgba16(i) => {
                width = i.width();
                height = i.height();
                format = TextureFormat::Rgba16Uint;

                let raw_data = i.into_raw();

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
                    let a = u16::max_value();

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
        )
    }

    /// Convert a [`Image`] to a [`DynamicImage`]. Usefull for editing image
    /// data. Not all [`TextureFormat`] are covered, therefore it will return an
    /// error if the format is unsupported. Supported formats are:
    /// - `TextureFormat::R8Unorm`
    /// - `TextureFormat::Rg8Unorm`
    /// - `TextureFormat::Rgba8UnormSrgb`
    ///
    /// To convert [`Image`] to a different format see: [`Image::convert`].
    pub fn try_into_dynamic(self) -> anyhow::Result<DynamicImage> {
        match self.texture_descriptor.format {
            TextureFormat::R8Unorm => ImageBuffer::from_raw(
                self.texture_descriptor.size.width,
                self.texture_descriptor.size.height,
                self.data,
            )
            .map(DynamicImage::ImageLuma8),
            TextureFormat::Rg8Unorm => ImageBuffer::from_raw(
                self.texture_descriptor.size.width,
                self.texture_descriptor.size.height,
                self.data,
            )
            .map(DynamicImage::ImageLumaA8),
            TextureFormat::Rgba8UnormSrgb => ImageBuffer::from_raw(
                self.texture_descriptor.size.width,
                self.texture_descriptor.size.height,
                self.data,
            )
            .map(DynamicImage::ImageRgba8),
            // Throw and error if conversion isn't supported
            texture_format => {
                return Err(anyhow!(
                    "Conversion into dynamic image not supported for {:?}.",
                    texture_format
                ))
            }
        }
        .ok_or_else(|| {
            anyhow!(
                "Failed to convert into {:?}.",
                self.texture_descriptor.format
            )
        })
    }
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

        let image = Image::from_dynamic(initial.clone(), true);

        // NOTE: Fails if `is_srbg = false` or the dynamic image is of the type rgb8.
        assert_eq!(initial, image.try_into_dynamic().unwrap());
    }
}
