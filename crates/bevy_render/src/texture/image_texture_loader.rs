use super::image_texture_conversion::image_to_texture;
use anyhow::Result;
use bevy_asset::{AssetLoader, LoadContext, LoadedAsset};
use bevy_utils::BoxedFuture;

/// Loader for images that can be read by the `image` crate.
#[derive(Clone, Default)]
pub struct ImageTextureLoader;

const FILE_EXTENSIONS: &[&str] = &["png", "dds", "tga", "jpg", "jpeg", "bmp"];

impl AssetLoader for ImageTextureLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<()>> {
        Box::pin(async move {
            // Find the image type we expect. A file with the extension "png" should
            // probably load as a PNG.

            let ext = load_context.path().extension().unwrap().to_str().unwrap();

            let img_format = image::ImageFormat::from_extension(ext)
                .ok_or_else(|| {
                    format!(
                    "Unexpected image format {:?} for file {}, this is an error in `bevy_render`.",
                    ext,
                    load_context.path().display()
                )
                })
                .unwrap();

            // Load the image in the expected format.
            // Some formats like PNG allow for R or RG textures too, so the texture
            // format needs to be determined. For RGB textures an alpha channel
            // needs to be added, so the image data needs to be converted in those
            // cases.

            let dyn_img = image::load_from_memory_with_format(bytes, img_format)?;

            load_context.set_default_asset(LoadedAsset::new(image_to_texture(dyn_img)));
            Ok(())
        })
    }

    fn extensions(&self) -> &[&str] {
        FILE_EXTENSIONS
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_supported_file_extensions() {
        for ext in FILE_EXTENSIONS {
            assert!(image::ImageFormat::from_extension(ext).is_some())
        }
    }
}
