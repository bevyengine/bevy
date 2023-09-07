use crate::texture::{Image, ImageFormat, ImageFormatSetting, ImageLoader, ImageLoaderSettings};
use bevy_asset::{
    anyhow::Error,
    saver::{AssetSaver, SavedAsset},
};
use futures_lite::{AsyncWriteExt, FutureExt};

pub struct CompressedImageSaver;

impl AssetSaver for CompressedImageSaver {
    type Asset = Image;

    type Settings = ();
    type OutputLoader = ImageLoader;

    fn save<'a>(
        &'a self,
        writer: &'a mut bevy_asset::io::Writer,
        image: SavedAsset<'a, Self::Asset>,
        _settings: &'a Self::Settings,
    ) -> bevy_utils::BoxedFuture<'a, std::result::Result<ImageLoaderSettings, Error>> {
        // PERF: this should live inside the future, but CompressorParams and Compressor are not Send / can't be owned by the BoxedFuture (which _is_ Send)
        let mut compressor_params = basis_universal::CompressorParams::new();
        compressor_params.set_basis_format(basis_universal::BasisTextureFormat::UASTC4x4);
        compressor_params.set_generate_mipmaps(true);
        let is_srgb = image.texture_descriptor.format.is_srgb();
        let color_space = if is_srgb {
            basis_universal::ColorSpace::Srgb
        } else {
            basis_universal::ColorSpace::Linear
        };
        compressor_params.set_color_space(color_space);
        compressor_params.set_uastc_quality_level(basis_universal::UASTC_QUALITY_DEFAULT);

        let mut source_image = compressor_params.source_image_mut(0);
        let size = image.size();
        source_image.init(&image.data, size.x as u32, size.y as u32, 4);

        let mut compressor = basis_universal::Compressor::new(4);
        // SAFETY: the CompressorParams are "valid" to the best of our knowledge. The basis-universal
        // library bindings note that invalid params might produce undefined behavior.
        unsafe {
            compressor.init(&compressor_params);
            compressor.process().unwrap();
        }
        let compressed_basis_data = compressor.basis_file().to_vec();
        async move {
            writer.write_all(&compressed_basis_data).await?;
            Ok(ImageLoaderSettings {
                format: ImageFormatSetting::Format(ImageFormat::Basis),
                is_srgb,
            })
        }
        .boxed()
    }
}
