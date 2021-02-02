use super::{Extent3d, Texture, TextureDimension, TextureFormat};

/// Helper method to convert a `DynamicImage` to a `Texture`
pub(crate) fn image_to_texture(dyn_img: image::DynamicImage) -> Texture {
    use bevy_core::AsBytes;

    let width;
    let height;

    let data: Vec<u8>;
    let format: TextureFormat;

    match dyn_img {
        image::DynamicImage::ImageLuma8(i) => {
            width = i.width();
            height = i.height();
            format = TextureFormat::R8Unorm;

            data = i.into_raw();
        }
        image::DynamicImage::ImageLumaA8(i) => {
            width = i.width();
            height = i.height();
            format = TextureFormat::Rg8Unorm;

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

            data = raw_data.as_slice().as_bytes().to_owned();
        }
        image::DynamicImage::ImageLumaA16(i) => {
            width = i.width();
            height = i.height();
            format = TextureFormat::Rg16Uint;

            let raw_data = i.into_raw();

            data = raw_data.as_slice().as_bytes().to_owned();
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

            data = raw_data.as_slice().as_bytes().to_owned();
        }
    }

    Texture::new(
        Extent3d::new(width, height, 1),
        TextureDimension::D2,
        data,
        format,
    )
}

/// Helper method to convert a `Texture` to a `DynamicImage`. Not all `Texture` formats are
/// covered, it will return `None` if the format is not supported
pub(crate) fn texture_to_image(texture: &Texture) -> Option<image::DynamicImage> {
    match texture.format {
        TextureFormat::R8Unorm => image::ImageBuffer::from_raw(
            texture.size.width,
            texture.size.height,
            texture.data.clone(),
        )
        .map(image::DynamicImage::ImageLuma8),
        TextureFormat::Rg8Unorm => image::ImageBuffer::from_raw(
            texture.size.width,
            texture.size.height,
            texture.data.clone(),
        )
        .map(image::DynamicImage::ImageLumaA8),
        TextureFormat::Rgba8UnormSrgb => image::ImageBuffer::from_raw(
            texture.size.width,
            texture.size.height,
            texture.data.clone(),
        )
        .map(image::DynamicImage::ImageRgba8),
        TextureFormat::Bgra8UnormSrgb => image::ImageBuffer::from_raw(
            texture.size.width,
            texture.size.height,
            texture.data.clone(),
        )
        .map(image::DynamicImage::ImageBgra8),
        _ => None,
    }
}
