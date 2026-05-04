#[cfg(feature = "compressed_image_saver")]
mod ctt;
#[cfg(feature = "compressed_image_saver")]
mod ctt_helpers;
#[cfg(all(
    feature = "compressed_image_saver_universal",
    not(feature = "compressed_image_saver")
))]
mod universal;

#[cfg(feature = "compressed_image_saver")]
use crate::compressed_image_saver::ctt::CompressedImageSaverCtt;
#[cfg(all(
    feature = "compressed_image_saver_universal",
    not(feature = "compressed_image_saver")
))]
use crate::compressed_image_saver::universal::CompressedImageSaverUniversal;
use crate::{Image, ImageLoader, ImageLoaderSettings};

use bevy_asset::{
    io::Writer,
    saver::{AssetSaver, SavedAsset},
    AssetPath,
};
use bevy_reflect::TypePath;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use wgpu_types::TextureFormat;

/// An [`AssetSaver`] for [`Image`] that compresses texture files.
///
/// Compressed textures use less GPU VRAM and are faster to sample.
///
/// # Platform support
///
/// Two mutually exclusive feature flags control which compression backend is used:
///
/// - **`compressed_image_saver`** — Uses the [`ctt`](https://github.com/cwfitzgerald/ctt)
///   library to compress textures into `BCn` or `ASTC` formats, output as KTX2. Requires a C++
///   compiler; see the [ctt readme](https://github.com/cwfitzgerald/ctt?tab=readme-ov-file#prebuilt-binaries).
///   Outputs BCn by default (for desktop targets). Set
///   `BEVY_COMPRESSED_IMAGE_SAVER_ASTC` to output `ASTC` instead (for mobile targets).
///
/// - **`compressed_image_saver_universal`** — Uses `basis-universal` to compress textures into UASTC
///   (Basis Universal) format. This is a GPU-agnostic supercompressed format that can be
///   transcoded at load time to whatever format the target GPU supports, making it suitable for
///   WebGPU and cross-platform distribution in a single file.
///
/// # Runtime feature flags
///
/// The compressed output must also be loadable at runtime. Enable the corresponding feature:
///
/// - **`ktx2` and `zstd`** — Required to load KTX2 files produced by `compressed_image_saver`.
/// - **`basis-universal`** — Required to load Basis Universal files produced by `compressed_image_saver_universal`.
///
/// # Compression format selection
///
/// The output format is chosen automatically based on the input texture's channel count and type:
///
/// | Input channels | Output format |
/// |---|---|
/// | 1-channel (`R8Unorm`) | BC4 |
/// | 1-channel snorm (`R8Snorm`) | BC4 snorm |
/// | 2-channel (`Rg8Unorm`) | BC5 |
/// | 2-channel snorm (`Rg8Snorm`) | BC5 snorm |
/// | HDR / float (e.g. `Rgba16Float`) | BC6H |
/// | 4-channel LDR (e.g. `Rgba8Unorm`) | BC7 |
/// | 4-channel sRGB (e.g. `Rgba8UnormSrgb`) | BC7 sRGB |
/// | Integer or high-precision (>16-bit) formats | Uncompressed KTX2 (passthrough) |
/// | Already compressed (`BCn`, `ASTC`, `ETC2`, `EAC`) | Re-encoded to the same format |
///
/// Depth, stencil, and video formats (`NV12`, `P010`) are not supported and will return
/// [`CompressedImageSaverError::UnsupportedFormat`].
///
/// # ASTC override
///
/// Set the `BEVY_COMPRESSED_IMAGE_SAVER_ASTC` environment variable to compress into `ASTC`
/// instead of `BCn`. `ASTC` is natively supported on mobile GPUs (Android, iOS) and some
/// desktop GPUs, while `BCn` is typically only supported on desktop GPUs.
///
/// The value specifies the block size. Larger blocks compress more aggressively (smaller
/// files, less VRAM) at the cost of quality. If set to an empty string or `1`, defaults
/// to `4x4`.
///
/// | Block size | Bits per pixel | Notes |
/// |---|---|---|
/// | `4x4` | 8.00 | Highest quality, same bit rate as BC7 |
/// | `6x6` | 3.56 | Good balance of quality and size |
/// | `8x8` | 2.00 | Aggressive, suitable for `base_color_texture` |
///
/// All 14 `ASTC` block sizes are supported: `4x4`, `5x4`, `5x5`, `6x5`, `6x6`, `8x5`,
/// `8x6`, `8x8`, `10x5`, `10x6`, `10x8`, `10x10`, `12x10`, `12x12`.
///
/// # Mipmap generation (`compressed_image_saver` only)
///
/// When using the `compressed_image_saver` backend, mipmaps are generated automatically
/// during compression. This prevents aliasing when textures are viewed at a distance and
/// increases GPU cache hits, improving rendering performance. Input images must have a
/// `mip_level_count` of 1 (i.e., no pre-existing mip chain); the compressor will produce
/// a full mip chain in the output.
///
/// The `compressed_image_saver_universal` backend does not generate mipmaps.
#[derive(TypePath, Default)]
#[expect(clippy::doc_markdown, reason = "clippy does not like unquoted BCn")]
pub struct CompressedImageSaver {
    #[cfg(feature = "compressed_image_saver")]
    inner: CompressedImageSaverCtt,
    #[cfg(all(
        feature = "compressed_image_saver_universal",
        not(feature = "compressed_image_saver")
    ))]
    inner: CompressedImageSaverUniversal,
}

impl AssetSaver for CompressedImageSaver {
    type Asset = Image;

    type Settings = CompressedImageSaverSettings;
    type OutputLoader = ImageLoader;
    type Error = CompressedImageSaverError;

    async fn save(
        &self,
        writer: &mut Writer,
        asset: SavedAsset<'_, '_, Self::Asset>,
        settings: &Self::Settings,
        asset_path: AssetPath<'_>,
    ) -> Result<ImageLoaderSettings, Self::Error> {
        self.inner.save(writer, asset, settings, asset_path).await
    }
}

/// Settings for [`CompressedImageSaver`].
#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct CompressedImageSaverSettings {
    /// The alpha mode the source image is authored in.
    ///
    /// Set this to match how the input texture stores its alpha channel. If the input does not
    /// match `output_alpha_mode`, the saver converts between the two before compression.
    ///
    /// Defaults to [`ImageCompressorAlphaMode::Straight`], which is how most image editors and asset pipelines
    /// produce textures.
    pub input_alpha_mode: ImageCompressorAlphaMode,
    /// The alpha mode the compressed output should use.
    ///
    /// With straight alpha, the RGB values of fully-transparent texels still consume endpoint
    /// precision in block-compressed formats and can bleed into neighboring opaque texels under
    /// texture filtering, producing colored fringes at transparent edges. Premultiplying forces
    /// transparent texels to zero RGB, which avoids both problems.
    ///
    /// Defaults to [`ImageCompressorAlphaMode::Premultiplied`]. Materials sampling this texture must be
    /// configured with `bevy_material::AlphaMode::Premultiplied` (or another premultiplied-blend
    /// mode) so the blend state matches.
    pub output_alpha_mode: ImageCompressorAlphaMode,
}

impl Default for CompressedImageSaverSettings {
    fn default() -> Self {
        Self {
            input_alpha_mode: ImageCompressorAlphaMode::Straight,
            output_alpha_mode: ImageCompressorAlphaMode::Premultiplied,
        }
    }
}

/// Alpha mode of an [`Image`] for use with [`CompressedImageSaver`].
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ImageCompressorAlphaMode {
    /// The image has an alpha channel that is stored independently of the RGB channels.
    Straight,
    /// The image has an alpha channel, and the RGB channels have been premultiplied by the alpha value.
    Premultiplied,
    /// The image has no alpha channel.
    Opaque,
}

/// Errors encountered when writing compressed images via [`CompressedImageSaver`].
#[non_exhaustive]
#[derive(Debug, Error, TypePath)]
pub enum CompressedImageSaverError {
    /// I/O error.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// The underlying compression library returned an error.
    #[error(transparent)]
    CompressionFailed(Box<dyn core::error::Error + Send + Sync>),
    /// Attempted to save an image with uninitialized data.
    #[error("Cannot compress an uninitialized image")]
    UninitializedImage,
    /// The texture format is not supported for compression.
    #[error("Unsupported texture format for compression: {0:?}")]
    UnsupportedFormat(TextureFormat),
}
