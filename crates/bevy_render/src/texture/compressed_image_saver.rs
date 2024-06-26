use crate::texture::{Image, ImageFormat, ImageFormatSetting, ImageLoader, ImageLoaderSettings};
use bevy_asset::saver::{AssetSaver, SavedAsset};
use futures_lite::AsyncWriteExt;
use thiserror::Error;

pub struct CompressedImageSaver;

#[non_exhaustive]
#[derive(Debug, Error)]
pub enum CompressedImageSaverError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

impl AssetSaver for CompressedImageSaver {
    type Asset = Image;

    type Settings = ();
    type OutputLoader = ImageLoader;
    type Error = CompressedImageSaverError;

    async fn save<'a>(
        &'a self,
        writer: &'a mut bevy_asset::io::Writer,
        image: SavedAsset<'a, Self::Asset>,
        _settings: &'a Self::Settings,
    ) -> Result<ImageLoaderSettings, Self::Error> {
        let is_srgb = image.texture_descriptor.format.is_srgb();

        let compressed_basis_data = {
            let mut compressor_params = basis_universal::CompressorParams::new();
            compressor_params.set_basis_format(basis_universal::BasisTextureFormat::UASTC4x4);
            compressor_params.set_generate_mipmaps(true);
            let color_space = if is_srgb {
                basis_universal::ColorSpace::Srgb
            } else {
                basis_universal::ColorSpace::Linear
            };
            compressor_params.set_color_space(color_space);
            compressor_params.set_uastc_quality_level(basis_universal::UASTC_QUALITY_DEFAULT);

            let mut source_image = compressor_params.source_image_mut(0);
            let size = image.size();
            source_image.init(&image.data, size.x, size.y, 4);

            let mut compressor = basis_universal::Compressor::new(4);
            // SAFETY: the CompressorParams are "valid" to the best of our knowledge. The basis-universal
            // library bindings note that invalid params might produce undefined behavior.
            unsafe {
                compressor.init(&compressor_params);
                compressor.process().unwrap();
            }
            compressor.basis_file().to_vec()
        };

        writer.write_all(&compressed_basis_data).await?;
        Ok(ImageLoaderSettings {
            format: ImageFormatSetting::Format(ImageFormat::Basis),
            is_srgb,
            sampler: image.sampler.clone(),
            asset_usage: image.asset_usage,
        })
    }
}
