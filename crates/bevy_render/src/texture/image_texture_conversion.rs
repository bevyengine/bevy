use crate::texture::{Image, TextureFormatPixelInfo};
use wgpu::{Extent3d, TextureDimension, TextureFormat};

// TODO: fix name?
/// Converts a [`DynamicImage`] to an [`Image`].
pub(crate) fn image_to_texture(dyn_img: image::DynamicImage) -> Image {
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
                // TODO use the array_chunks method once stabilised
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
        image::DynamicImage::ImageRgba16(i) => {
            width = i.width();
            height = i.height();
            format = TextureFormat::Rgba16Uint;

            let raw_data = i.into_raw();

            data = cast_slice(&raw_data).to_owned();
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

/// Converts an [`Image`] to a [`DynamicImage`]. Not all [`TextureFormat`] are
/// covered, therefore it will return `None` if the format is unsupported.
pub(crate) fn texture_to_image(texture: &Image) -> Option<image::DynamicImage> {
    match texture.texture_descriptor.format {
        TextureFormat::R8Unorm => image::ImageBuffer::from_raw(
            texture.texture_descriptor.size.width,
            texture.texture_descriptor.size.height,
            texture.data.clone(),
        )
        .map(image::DynamicImage::ImageLuma8),
        TextureFormat::Rg8Unorm => image::ImageBuffer::from_raw(
            texture.texture_descriptor.size.width,
            texture.texture_descriptor.size.height,
            texture.data.clone(),
        )
        .map(image::DynamicImage::ImageLumaA8),
        TextureFormat::Rgba8UnormSrgb => image::ImageBuffer::from_raw(
            texture.texture_descriptor.size.width,
            texture.texture_descriptor.size.height,
            texture.data.clone(),
        )
        .map(image::DynamicImage::ImageRgba8),
        TextureFormat::Bgra8UnormSrgb => image::ImageBuffer::from_raw(
            texture.texture_descriptor.size.width,
            texture.texture_descriptor.size.height,
            texture.data.clone(),
        )
        .map(image::DynamicImage::ImageBgra8),
        _ => None,
    }
}
