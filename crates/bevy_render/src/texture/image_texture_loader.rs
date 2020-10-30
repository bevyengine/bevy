use super::{Texture, TextureFormat};
use anyhow::Result;
use bevy_asset::{AssetLoader, LoadContext, LoadedAsset};
use bevy_math::Vec2;
use bevy_utils::BoxedFuture;

/// Loader for images that can be read by the `image` crate.
///
/// Reads only PNG images for now.
#[derive(Clone, Default)]
pub struct ImageTextureLoader;

impl AssetLoader for ImageTextureLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<()>> {
        Box::pin(async move {
            use bevy_core::AsBytes;

            // Find the image type we expect. A file with the extension "png" should
            // probably load as a PNG.

            let ext = load_context.path().extension().unwrap().to_str().unwrap();

            // NOTE: If more formats are added they can be added here.
            let img_format = if ext.eq_ignore_ascii_case("png") {
                image::ImageFormat::Png
            } else {
                panic!(
                    "Unexpected image format {:?} for file {}, this is an error in `bevy_render`.",
                    ext,
                    load_context.path().display()
                )
            };

            // Load the image in the expected format.
            // Some formats like PNG allow for R or RG textures too, so the texture
            // format needs to be determined. For RGB textures an alpha channel
            // needs to be added, so the image data needs to be converted in those
            // cases.

            let dyn_img = image::load_from_memory_with_format(bytes, img_format)?;

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
                    let i = image::DynamicImage::ImageRgb8(i).into_rgba();
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
                    let i = image::DynamicImage::ImageBgr8(i).into_bgra();

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

            let texture = Texture::new(Vec2::new(width as f32, height as f32), data, format);
            load_context.set_default_asset(LoadedAsset::new(texture));
            Ok(())
        })
    }

    fn extensions(&self) -> &[&str] {
        static EXTENSIONS: &[&str] = &["png"];
        EXTENSIONS
    }
}
