//! Asset saver and processor for Basis Universal KTX2 textures.

use bevy_asset::{
    processor::LoadTransformAndSave, saver::AssetSaver, transformer::IdentityAssetTransformer,
    AsyncWriteExt,
};
use bevy_image::{Image, ImageLoader};
use bevy_reflect::TypePath;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use basisu_c_sys::extra::{BasisuEncodeError, BasisuEncoder, BasisuEncoderParams};

use crate::loader::{BasisuLoader, BasisuLoaderSettings};

/// Basis universal asset processor.
pub type BasisuProcessor =
    LoadTransformAndSave<ImageLoader, IdentityAssetTransformer<Image>, BasisuSaver>;

/// Basis universal texture saver.
#[derive(TypePath)]
pub struct BasisuSaver;

/// Basis universal texture saver settings.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct BasisuSaverSettings {
    /// Basisu encoder params.
    /// See the `BU_COMP_FLAGS_*` in [`basisu_c_sys`] if you want more controls,
    /// like mipmap generation.
    pub params: BasisuEncoderParams,
}

impl Default for BasisuSaverSettings {
    fn default() -> Self {
        Self {
            params: BasisuEncoderParams::new_with_srgb_defaults(
                basisu_c_sys::BasisTextureFormat::XuastcLdr4x4,
            ),
        }
    }
}

/// An error when encoding an image using [`BasisuSaver`].
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum BasisuSaverError {
    /// An error occurred while trying to load the bytes.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// An error occurred while trying to encode the image.
    #[error(transparent)]
    BasisuEncodeError(#[from] BasisuEncodeError),
}

impl AssetSaver for BasisuSaver {
    type Asset = Image;
    type Settings = BasisuSaverSettings;
    type OutputLoader = BasisuLoader;
    type Error = BasisuSaverError;

    async fn save(
        &self,
        writer: &mut bevy_asset::io::Writer,
        asset: bevy_asset::saver::SavedAsset<'_, '_, Self::Asset>,
        settings: &Self::Settings,
        _asset_path: bevy_asset::AssetPath<'_>,
    ) -> Result<<Self::OutputLoader as bevy_asset::AssetLoader>::Settings, Self::Error> {
        let mut encoder = BasisuEncoder::new();
        encoder.set_image(basisu_c_sys::extra::SourceImage {
            data: asset.data.as_deref().unwrap_or(&[]),
            texture_descriptor: &asset.texture_descriptor,
            texture_view_descriptor: &asset.texture_view_descriptor,
        })?;
        let result = encoder.compress(settings.params)?;
        writer.write_all(&result).await?;

        Ok(BasisuLoaderSettings {
            asset_usage: asset.asset_usage,
            sampler: asset.sampler.clone(),
            is_srgb: None,
            force_transcode_target: None,
        })
    }
}
