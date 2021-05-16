use std::convert::TryFrom;
use thiserror::Error;

use super::{Extent3d, Texture, TextureDimension, TextureFormat};

impl From<image::DynamicImage> for Texture {
    fn from(dyn_img: image::DynamicImage) -> Self {
        use bevy_core::cast_slice;
        let width;
        let height;

        let data: Vec<u8>;
        let format: TextureFormat;

        match dyn_img {
            image::DynamicImage::ImageLuma8(i) => {
                let i = image::DynamicImage::ImageLuma8(i).into_rgba8();
                width = i.width();
                height = i.height();
                format = TextureFormat::Rgba8UnormSrgb;

                data = i.into_raw();
            }
            image::DynamicImage::ImageLumaA8(i) => {
                let i = image::DynamicImage::ImageLumaA8(i).into_rgba8();
                width = i.width();
                height = i.height();
                format = TextureFormat::Rgba8UnormSrgb;

                data = i.into_raw();
            }
            image::DynamicImage::ImageRgb8(i) => {
                let i = image::DynamicImage::ImageRgb8(i).into_rgba8();
                width = i.width();
                height = i.height();
                format = TextureFormat::Rgba8UnormSrgb;

                data = i.into_raw();
            }
            image::DynamicImage::ImageRgba8(i) => {
                width = i.width();
                height = i.height();
                format = TextureFormat::Rgba8UnormSrgb;

                data = i.into_raw();
            }
            image::DynamicImage::ImageBgr8(i) => {
                let i = image::DynamicImage::ImageBgr8(i).into_bgra8();

                width = i.width();
                height = i.height();
                format = TextureFormat::Bgra8UnormSrgb;

                data = i.into_raw();
            }
            image::DynamicImage::ImageBgra8(i) => {
                width = i.width();
                height = i.height();
                format = TextureFormat::Bgra8UnormSrgb;

                data = i.into_raw();
            }
            image::DynamicImage::ImageLuma16(i) => {
                width = i.width();
                height = i.height();
                format = TextureFormat::R16Uint;
                let raw_data = i.into_raw();

                data = cast_slice(&raw_data).to_owned();
            }
            image::DynamicImage::ImageLumaA16(i) => {
                width = i.width();
                height = i.height();
                format = TextureFormat::Rg16Uint;

                let raw_data = i.into_raw();

                data = cast_slice(&raw_data).to_owned();
            }

            image::DynamicImage::ImageRgb16(image) => {
                width = image.width();
                height = image.height();
                format = TextureFormat::Rgba16Uint;

                let mut local_data =
                    Vec::with_capacity(width as usize * height as usize * format.pixel_size());

                for pixel in image.into_raw().chunks_exact(3) {
                    // TODO unsafe_get in release builds?
                    let r = pixel[0];
                    let g = pixel[1];
                    let b = pixel[2];
                    let a = u16::MAX;

                    local_data.extend_from_slice(&r.to_ne_bytes());
                    local_data.extend_from_slice(&g.to_ne_bytes());
                    local_data.extend_from_slice(&b.to_ne_bytes());
                    local_data.extend_from_slice(&a.to_ne_bytes());
                }

                data = local_data;
            }
            image::DynamicImage::ImageRgba16(i) => {
                width = i.width();
                height = i.height();
                format = TextureFormat::Rgba16Uint;

                let raw_data = i.into_raw();

                data = cast_slice(&raw_data).to_owned();
            }
        }

        Texture::new(
            Extent3d::new(width, height, 1),
            TextureDimension::D2,
            data,
            format,
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
pub enum TextureConversionError {
    #[error("Unsupported texture format")]
    UnsupportedFormat,
    #[error("Invalid texture size")]
    InvalidSize,
}

impl TryFrom<Texture> for image::DynamicImage {
    type Error = TextureConversionError;

    fn try_from(texture: Texture) -> Result<Self, Self::Error> {
        match texture.format {
            TextureFormat::R8Unorm => {
                image::ImageBuffer::from_raw(texture.size.width, texture.size.height, texture.data)
                    .map(image::DynamicImage::ImageLuma8)
                    .ok_or(TextureConversionError::InvalidSize)
            }
            TextureFormat::Rg8Unorm => {
                image::ImageBuffer::from_raw(texture.size.width, texture.size.height, texture.data)
                    .map(image::DynamicImage::ImageLumaA8)
                    .ok_or(TextureConversionError::InvalidSize)
            }
            TextureFormat::Rgba8UnormSrgb => {
                image::ImageBuffer::from_raw(texture.size.width, texture.size.height, texture.data)
                    .map(image::DynamicImage::ImageRgba8)
                    .ok_or(TextureConversionError::InvalidSize)
            }
            TextureFormat::Bgra8UnormSrgb => {
                image::ImageBuffer::from_raw(texture.size.width, texture.size.height, texture.data)
                    .map(image::DynamicImage::ImageBgra8)
                    .ok_or(TextureConversionError::InvalidSize)
            }
            _ => Err(TextureConversionError::UnsupportedFormat),
        }
    }
}
