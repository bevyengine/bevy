use crate::{Image, ImageFormat, ImageFormatSetting, ImageLoader, ImageLoaderSettings};

use bevy_asset::saver::{AssetSaver, SavedAsset};
use futures_lite::AsyncWriteExt;
use thiserror::Error;
use wgpu_types::TextureFormat;

pub struct CompressedImageSaver;

#[non_exhaustive]
#[derive(Debug, Error)]
pub enum CompressedImageSaverError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("Cannot compress an uninitialized image")]
    UninitializedImage,
    #[error("Cannot compress {0:?}")]
    UnsupportedFormat(TextureFormat),
}

impl AssetSaver for CompressedImageSaver {
    type Asset = Image;

    type Settings = ();
    type OutputLoader = ImageLoader;
    type Error = CompressedImageSaverError;

    async fn save(
        &self,
        writer: &mut bevy_asset::io::Writer,
        image: SavedAsset<'_, Self::Asset>,
        _settings: &Self::Settings,
    ) -> Result<ImageLoaderSettings, Self::Error> {
        if image.data.is_none() {
            return Err(CompressedImageSaverError::UninitializedImage);
        }

        let source_format = image.texture_descriptor.format;
        let is_srgb = source_format.is_srgb();

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
            compressor_params.set_create_ktx2_file(true);

            match source_format {
                TextureFormat::R32Float | TextureFormat::Rg32Float | TextureFormat::Rgba32Float => {
                    compressor_params.set_hdr(true);
                    compressor_params
                        .set_basis_format(basis_universal::BasisTextureFormat::UASTC_HDR_4x4);
                    compressor_params.set_hdr_favor_uastc(true);
                    compressor_params.set_hdr_mode(basis_universal::HdrMode::HDR_UASTC_HDR_4x4);
                    let mut source_image = compressor_params.source_hdr_image_mut(0);
                    let size = image.size();
                    let data = image.data.as_ref().unwrap().as_slice();
                    let channel_count = match source_format {
                        TextureFormat::R32Float => 1,
                        TextureFormat::Rg32Float => 2,
                        TextureFormat::Rgba32Float => 4,
                        _ => unreachable!(),
                    };
                    source_image.init(data, size.x, size.y, channel_count);
                }
                TextureFormat::Rgba8Unorm | TextureFormat::Rgba8UnormSrgb => {
                    let mut source_image = compressor_params.source_image_mut(0);
                    let size = image.size();
                    let data = image.data.as_ref().unwrap().as_slice();
                    source_image.init(data, size.x, size.y, 4);
                }
                format => return Err(CompressedImageSaverError::UnsupportedFormat(format)),
            }

            let mut compressor = basis_universal::Compressor::new(4);
            #[expect(
                unsafe_code,
                reason = "The basis-universal compressor cannot be interacted with except through unsafe functions"
            )]
            // SAFETY: the CompressorParams are "valid" to the best of our knowledge. The basis-universal
            // library bindings note that invalid params might produce undefined behavior.
            unsafe {
                compressor.init(&compressor_params);
                compressor.process().unwrap();
            }
            compressor.ktx2_file().to_vec()
        };

        writer.write_all(&compressed_basis_data).await?;
        Ok(ImageLoaderSettings {
            format: ImageFormatSetting::Format(ImageFormat::Ktx2),
            is_srgb,
            sampler: image.sampler.clone(),
            asset_usage: image.asset_usage,
        })
    }
}
