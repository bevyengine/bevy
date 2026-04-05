//! Asset loader for Basis Universal KTX2 textures.
//!
//! The file extension must be `.basisu.ktx2` to use this loader. All basis universal compressed formats (ETC1S, UASTC, XUASTC) are supported. Zstd supercompression is always supported. No support for `.basis` files.
//!
//! Default transcode target selection:
//!
//! | BasisU formats                 | Target selection                                               |
//! | ------------------------------ | -------------------------------------------------------------- |
//! | ETC1S                          | Etc2Rgba8/Etc2Rgb8/EacRg11/EacR11 > Bc7Rgba/Bc5Rg/Bc4R > Rgba8 |
//! | UASTC_LDR, ASTC_LDR, XUASTC_LDR| Astc > Bc7Rgba > Etc2Rgba8/Etc2Rgb8/EacRg11/EacR11 > Rgba8     |
//! | UASTC_HDR, ASTC_HDR            | Astc > Bc6hRgbUfloat > Rgba16Float                             |

use basisu_c_sys::extra::{BasisuTranscodeError, BasisuTranscoder, SupportedTextureCompression};
use bevy_asset::{AssetLoader, RenderAssetUsages};
use bevy_image::{CompressedImageFormats, Image, ImageSampler};
use bevy_reflect::TypePath;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Basis Universal texture loader.
#[derive(TypePath)]
pub struct BasisuLoader {
    supported_compressed_formats: SupportedTextureCompression,
}

impl BasisuLoader {
    /// Create a basisu loader from the supported compressed formats.
    pub fn new(supported_formats: CompressedImageFormats) -> Self {
        let mut supported_compressed_formats = SupportedTextureCompression::empty();
        if supported_formats.contains(CompressedImageFormats::ASTC_LDR) {
            supported_compressed_formats |= SupportedTextureCompression::ASTC_LDR;
        }
        if supported_formats.contains(CompressedImageFormats::ASTC_HDR) {
            supported_compressed_formats |= SupportedTextureCompression::ASTC_HDR;
        }
        if supported_formats.contains(CompressedImageFormats::BC) {
            supported_compressed_formats |= SupportedTextureCompression::BC;
        }
        if supported_formats.contains(CompressedImageFormats::ETC2) {
            supported_compressed_formats |= SupportedTextureCompression::ETC2;
        }
        Self {
            supported_compressed_formats,
        }
    }
}

/// Settings for loading an [`Image`] using an [`BasisuLoader`].
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct BasisuLoaderSettings {
    /// [`ImageSampler`] to use when rendering - this does
    /// not affect the loading of the image data.
    pub sampler: ImageSampler,
    /// Where the asset will be used - see the docs on
    /// [`RenderAssetUsages`] for details.
    pub asset_usage: RenderAssetUsages,
    /// Whether the texture should be created as sRGB format.
    ///
    /// If `None`, it will be determined by the KTX2 data format descriptor transfer function.
    pub is_srgb: Option<bool>,
    /// Forcibly transcode to a specific target format. If `None` the target format is selected automatically.
    ///
    /// It will fail to load if the target format is not supported by the device or it can't be transcoded by Basis Universal.
    pub force_transcode_target: Option<basisu_c_sys::TranscodeTargetFormat>,
}

/// An error when loading an image using [`BasisuLoader`].
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum BasisuLoaderError {
    /// An error occurred while trying to load the image bytes.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// An error occurred while trying to transcode basisu textures.
    #[error(transparent)]
    TranscodeError(#[from] BasisuTranscodeError),
}

impl AssetLoader for BasisuLoader {
    type Asset = Image;
    type Settings = BasisuLoaderSettings;
    type Error = BasisuLoaderError;

    async fn load(
        &self,
        reader: &mut dyn bevy_asset::io::Reader,
        settings: &Self::Settings,
        _load_context: &mut bevy_asset::LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut data = Vec::new();
        reader.read_to_end(&mut data).await?;
        let src_bytes = data.len();

        let _span = bevy_log::info_span!("transcoding basisu texture").entered();
        let time = if bevy_log::STATIC_MAX_LEVEL >= bevy_log::Level::DEBUG {
            Some(bevy_platform::time::Instant::now())
        } else {
            None
        };
        let mut transcoder = BasisuTranscoder::new();
        let info = transcoder.prepare(
            &data,
            self.supported_compressed_formats,
            basisu_c_sys::extra::ChannelType::Auto,
        )?;

        let out_image = transcoder.transcode(settings.force_transcode_target, settings.is_srgb)?;

        if bevy_log::STATIC_MAX_LEVEL >= bevy_log::Level::DEBUG {
            bevy_log::debug!(
	            "Transcoded a basisu texture {:?} -> {:?}, {:?}kb -> {:?}kb, preferred_target {:?}, extents {:?}, levels {:?}, view_dimension {:?}, in {:?}",
	            info.basis_format,
	            out_image.texture_descriptor.format,
	            src_bytes as f32 / 1000.0,
	            out_image.data.as_ref().unwrap().len() as f32 / 1000.0,
	            info.preferred_target,
	            out_image.texture_descriptor.size,
	            info.levels,
	            out_image
	                .texture_view_descriptor
	                .as_ref()
	                .unwrap()
	                .dimension
	                .unwrap(),
	            time.unwrap().elapsed(),
	        );
        }

        Ok(Image {
            data: out_image.data,
            data_order: out_image.data_order,
            texture_descriptor: out_image.texture_descriptor,
            texture_view_descriptor: out_image.texture_view_descriptor,
            copy_on_resize: false,
            sampler: settings.sampler.clone(),
            asset_usage: settings.asset_usage,
        })
    }

    fn extensions(&self) -> &[&str] {
        &["basisu.ktx2"]
    }
}
