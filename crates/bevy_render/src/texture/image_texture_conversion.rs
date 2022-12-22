use crate::texture::Image;
use anyhow::anyhow;
use image::{buffer::ConvertBuffer, DynamicImage, ImageBuffer, Luma, Rgba};
use wgpu::{Extent3d, TextureDimension, TextureFormat};

impl Image {
    /// Converts a [`DynamicImage`] to an [`Image`].
    pub fn from_dynamic(dyn_img: DynamicImage, is_srgb: bool) -> Image {
        let width = dyn_img.width();
        let height = dyn_img.height();

        let data: Vec<u8>;
        let format: TextureFormat;

        match dyn_img {
            DynamicImage::ImageLuma8(i) => {
                format = TextureFormat::R8Unorm;
                data = i.into_raw();
            }
            DynamicImage::ImageLumaA8(i) => {
                format = TextureFormat::R8Unorm;
                data = ConvertBuffer::<ImageBuffer<Luma<u8>, Vec<u8>>>::convert(&i).into_raw();
            }
            DynamicImage::ImageRgb8(i) => {
                if is_srgb {
                    format = TextureFormat::Rgba8UnormSrgb;
                } else {
                    format = TextureFormat::Rgba8Unorm;
                }
                data = ConvertBuffer::<ImageBuffer<Rgba<u8>, Vec<u8>>>::convert(&i).into_raw();
            }
            DynamicImage::ImageRgba8(i) => {
                if is_srgb {
                    format = TextureFormat::Rgba8UnormSrgb;
                } else {
                    format = TextureFormat::Rgba8Unorm;
                }
                data = i.into_raw();
            }
            _ => {
                let image = dyn_img.into_rgba8();
                if is_srgb {
                    format = TextureFormat::Rgba8UnormSrgb;
                } else {
                    format = TextureFormat::Rgba8Unorm;
                }

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
