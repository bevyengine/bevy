use crate::{Image, ImageFormat, ImageFormatSetting, ImageLoader, ImageLoaderSettings};

use bevy_asset::{
    io::Writer,
    saver::{AssetSaver, SavedAsset},
    AssetPath,
};
use bevy_reflect::TypePath;
use futures_lite::AsyncWriteExt;
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
///   library to compress textures into BCn or ASTC formats, output as KTX2. Requires a C++
///   compiler; see the [ctt readme](https://github.com/cwfitzgerald/ctt?tab=readme-ov-file#prebuilt-binaries).
///   Outputs BCn by default (for desktop targets). Set
///   `BEVY_COMPRESSED_IMAGE_SAVER_ASTC` to output ASTC instead (for mobile targets).
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
/// | Already compressed (BCn, ASTC, ETC2, EAC) | Re-encoded to the same format |
///
/// Depth, stencil, and video formats (`NV12`, `P010`) are not supported and will return
/// [`CompressedImageSaverError::UnsupportedFormat`].
///
/// # ASTC override
///
/// Set the `BEVY_COMPRESSED_IMAGE_SAVER_ASTC` environment variable to compress into ASTC
/// instead of BCn. ASTC is natively supported on mobile GPUs (Android, iOS) and some
/// desktop GPUs, while BCn is typically only supported on desktop GPUs.
///
/// The value specifies the block size. Larger blocks compress more aggressively (smaller
/// files, less VRAM) at the cost of quality. If set to an empty string or `1`, defaults
/// to `4x4`.
///
/// | Block size | Bits per pixel | Notes |
/// |---|---|---|
/// | `4x4` | 8.00 | Highest quality, same bit rate as BC7 |
/// | `6x6` | 3.56 | Good balance of quality and size |
/// | `8x8` | 2.00 | Aggressive, suitable for base_color_texture |
///
/// All 14 ASTC block sizes are supported: `4x4`, `5x4`, `5x5`, `6x5`, `6x6`, `8x5`,
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
#[derive(TypePath)]
pub struct CompressedImageSaver;

/// Errors encountered when writing compressed images via [`CompressedImageSaver`].
#[non_exhaustive]
#[derive(Debug, Error, TypePath)]
pub enum CompressedImageSaverError {
    /// I/O error.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// The underlying compression library returned an error.
    #[error(transparent)]
    CompressionFailed(Box<dyn std::error::Error + Send + Sync>),
    /// Attempted to save an image with uninitialized data.
    #[error("Cannot compress an uninitialized image")]
    UninitializedImage,
    /// The texture format is not supported for compression.
    #[error("Unsupported texture format for compression: {0:?}")]
    UnsupportedFormat(TextureFormat),
}

impl AssetSaver for CompressedImageSaver {
    type Asset = Image;

    type Settings = ();
    type OutputLoader = ImageLoader;
    type Error = CompressedImageSaverError;

    #[cfg(feature = "compressed_image_saver")]
    async fn save(
        &self,
        writer: &mut Writer,
        image: SavedAsset<'_, '_, Self::Asset>,
        _settings: &Self::Settings,
        _asset_path: AssetPath<'_>,
    ) -> Result<ImageLoaderSettings, Self::Error> {
        let Some(ref data) = image.data else {
            return Err(CompressedImageSaverError::UninitializedImage);
        };

        if image.texture_descriptor.mip_level_count != 1 {
            return Err(CompressedImageSaverError::CompressionFailed(
                "Expected texture_descriptor.mip_level_count to be 1".into(),
            ));
        }

        let input_format = map_to_ctt_texture_format(image.texture_descriptor.format)?;
        let output_format = choose_ctt_compressed_format(image.texture_descriptor.format)?;

        let is_srgb = image.texture_descriptor.format.is_srgb();
        let color_space = if is_srgb {
            ctt::ColorSpace::Srgb
        } else {
            ctt::ColorSpace::Linear
        };

        let is_cubemap = matches!(
            image.texture_view_descriptor,
            Some(wgpu_types::TextureViewDescriptor {
                dimension: Some(wgpu_types::TextureViewDimension::Cube),
                ..
            })
        );

        let bytes_per_pixel =
            crate::TextureFormatPixelInfo::pixel_size(&image.texture_descriptor.format).map_err(
                |_| CompressedImageSaverError::UnsupportedFormat(image.texture_descriptor.format),
            )? as u32;

        let surfaces = data
            .chunks_exact((image.width() * image.height() * bytes_per_pixel) as usize)
            .map(|layer_data| {
                vec![ctt::Surface {
                    data: layer_data.to_vec(),
                    width: image.width(),
                    height: image.height(),
                    stride: image.width() * bytes_per_pixel,
                    format: input_format,
                    color_space,
                    alpha: ctt::AlphaMode::Straight, // TODO: User-configurable
                }]
            })
            .collect();
        let ctt_image = ctt::Image {
            surfaces,
            is_cubemap,
        };

        let settings = ctt::ConvertSettings {
            format: Some(output_format),
            container: ctt::Container::ktx2_zstd(0),
            quality: ctt::Quality::default(),
            output_color_space: None,
            output_alpha: Some(ctt::AlphaMode::Premultiplied), // TODO: User-configurable
            swizzle: None,
            mipmap: true,
            mipmap_count: None,
            mipmap_filter: ctt::MipmapFilter::default(),
            allow_lossy: false,
            encoder_settings: None,
            registry: None,
        };

        let output = ctt::convert(ctt_image, settings)
            .map_err(|e| CompressedImageSaverError::CompressionFailed(Box::new(e)))?;
        let ctt::PipelineOutput::Encoded(compressed_bytes) = &output else {
            return Err(CompressedImageSaverError::CompressionFailed(
                "Expected encoded output from ctt".into(),
            ));
        };

        writer.write_all(compressed_bytes).await?;

        Ok(ImageLoaderSettings {
            format: ImageFormatSetting::Format(ImageFormat::Ktx2),
            is_srgb,
            sampler: image.sampler.clone(),
            asset_usage: image.asset_usage,
            texture_format: None,
            array_layout: None,
        })
    }

    #[cfg(feature = "compressed_image_saver_universal")]
    async fn save(
        &self,
        writer: &mut Writer,
        image: SavedAsset<'_, '_, Self::Asset>,
        _settings: &Self::Settings,
        _asset_path: AssetPath<'_>,
    ) -> Result<ImageLoaderSettings, Self::Error> {
        let is_srgb = image.texture_descriptor.format.is_srgb();

        let compressed_basis_data = {
            let mut compressor_params = basis_universal::CompressorParams::new();
            compressor_params.set_basis_format(basis_universal::BasisTextureFormat::UASTC4x4);
            compressor_params.set_generate_mipmaps(true);
            let color_space = if is_srgb {
                basis_universal::ColorSpace::Srgb
            } else {
                compressor_params.set_no_selector_rdo(true);
                basis_universal::ColorSpace::Linear
            };
            compressor_params.set_color_space(color_space);
            compressor_params.set_uastc_quality_level(basis_universal::UASTC_QUALITY_DEFAULT);

            let mut source_image = compressor_params.source_image_mut(0);
            let size = image.size();
            let Some(ref data) = image.data else {
                return Err(CompressedImageSaverError::UninitializedImage);
            };
            source_image.init(data, size.x, size.y, 4);

            let mut compressor = basis_universal::Compressor::new(4);
            #[expect(
                unsafe_code,
                reason = "The basis-universal compressor cannot be interacted with except through unsafe functions"
            )]
            // SAFETY: the CompressorParams are "valid" to the best of our knowledge. The basis-universal
            // library bindings note that invalid params might produce undefined behavior.
            unsafe {
                compressor.init(&compressor_params);
                compressor
                    .process()
                    .map_err(|e| CompressedImageSaverError::CompressionFailed(Box::new(e)))?;
            }
            compressor.basis_file().to_vec()
        };

        writer.write_all(&compressed_basis_data).await?;
        Ok(ImageLoaderSettings {
            format: ImageFormatSetting::Format(ImageFormat::Basis),
            is_srgb,
            sampler: image.sampler.clone(),
            asset_usage: image.asset_usage,
            texture_format: None,
            array_layout: None,
        })
    }
}

/// Returns `Some((unorm, srgb, hdr))` ASTC format triple if the env var is set, `None` otherwise.
#[cfg(feature = "compressed_image_saver")]
fn parse_astc_env_var(
) -> Result<Option<(ktx2::Format, ktx2::Format, ktx2::Format)>, CompressedImageSaverError> {
    use ktx2::Format;

    let val = match std::env::var("BEVY_COMPRESSED_IMAGE_SAVER_ASTC") {
        Ok(v) => v,
        Err(std::env::VarError::NotPresent) => return Ok(None),
        Err(std::env::VarError::NotUnicode(_)) => return Ok(None),
    };

    let val = val.trim();
    let (unorm, srgb, hdr) = match val {
        "" | "1" | "4x4" => (
            Format::ASTC_4x4_UNORM_BLOCK,
            Format::ASTC_4x4_SRGB_BLOCK,
            Format::ASTC_4x4_SFLOAT_BLOCK,
        ),
        "5x4" => (
            Format::ASTC_5x4_UNORM_BLOCK,
            Format::ASTC_5x4_SRGB_BLOCK,
            Format::ASTC_5x4_SFLOAT_BLOCK,
        ),
        "5x5" => (
            Format::ASTC_5x5_UNORM_BLOCK,
            Format::ASTC_5x5_SRGB_BLOCK,
            Format::ASTC_5x5_SFLOAT_BLOCK,
        ),
        "6x5" => (
            Format::ASTC_6x5_UNORM_BLOCK,
            Format::ASTC_6x5_SRGB_BLOCK,
            Format::ASTC_6x5_SFLOAT_BLOCK,
        ),
        "6x6" => (
            Format::ASTC_6x6_UNORM_BLOCK,
            Format::ASTC_6x6_SRGB_BLOCK,
            Format::ASTC_6x6_SFLOAT_BLOCK,
        ),
        "8x5" => (
            Format::ASTC_8x5_UNORM_BLOCK,
            Format::ASTC_8x5_SRGB_BLOCK,
            Format::ASTC_8x5_SFLOAT_BLOCK,
        ),
        "8x6" => (
            Format::ASTC_8x6_UNORM_BLOCK,
            Format::ASTC_8x6_SRGB_BLOCK,
            Format::ASTC_8x6_SFLOAT_BLOCK,
        ),
        "8x8" => (
            Format::ASTC_8x8_UNORM_BLOCK,
            Format::ASTC_8x8_SRGB_BLOCK,
            Format::ASTC_8x8_SFLOAT_BLOCK,
        ),
        "10x5" => (
            Format::ASTC_10x5_UNORM_BLOCK,
            Format::ASTC_10x5_SRGB_BLOCK,
            Format::ASTC_10x5_SFLOAT_BLOCK,
        ),
        "10x6" => (
            Format::ASTC_10x6_UNORM_BLOCK,
            Format::ASTC_10x6_SRGB_BLOCK,
            Format::ASTC_10x6_SFLOAT_BLOCK,
        ),
        "10x8" => (
            Format::ASTC_10x8_UNORM_BLOCK,
            Format::ASTC_10x8_SRGB_BLOCK,
            Format::ASTC_10x8_SFLOAT_BLOCK,
        ),
        "10x10" => (
            Format::ASTC_10x10_UNORM_BLOCK,
            Format::ASTC_10x10_SRGB_BLOCK,
            Format::ASTC_10x10_SFLOAT_BLOCK,
        ),
        "12x10" => (
            Format::ASTC_12x10_UNORM_BLOCK,
            Format::ASTC_12x10_SRGB_BLOCK,
            Format::ASTC_12x10_SFLOAT_BLOCK,
        ),
        "12x12" => (
            Format::ASTC_12x12_UNORM_BLOCK,
            Format::ASTC_12x12_SRGB_BLOCK,
            Format::ASTC_12x12_SFLOAT_BLOCK,
        ),
        other => {
            return Err(CompressedImageSaverError::CompressionFailed(
                format!("Invalid BEVY_COMPRESSED_IMAGE_SAVER_ASTC block size: {other:?}. \
                    Expected one of: 4x4, 5x4, 5x5, 6x5, 6x6, 8x5, 8x6, 8x8, 10x5, 10x6, 10x8, 10x10, 12x10, 12x12")
                    .into(),
            ));
        }
    };

    Ok(Some((unorm, srgb, hdr)))
}

#[cfg(feature = "compressed_image_saver")]
fn choose_ctt_compressed_format(
    input: TextureFormat,
) -> Result<ctt::TargetFormat, CompressedImageSaverError> {
    use ktx2::Format;

    let astc_block = parse_astc_env_var()?;

    let format = match input {
        // 1-channel snorm
        TextureFormat::R8Snorm => {
            if let Some((unorm, _, _)) = astc_block {
                unorm
            } else {
                Format::BC4_SNORM_BLOCK
            }
        }

        // 1-channel
        TextureFormat::R8Unorm => {
            if let Some((unorm, _, _)) = astc_block {
                unorm
            } else {
                Format::BC4_UNORM_BLOCK
            }
        }

        // 2-channel snorm
        TextureFormat::Rg8Snorm => {
            if let Some((unorm, _, _)) = astc_block {
                unorm
            } else {
                Format::BC5_SNORM_BLOCK
            }
        }

        // 2-channel
        TextureFormat::Rg8Unorm => {
            if let Some((unorm, _, _)) = astc_block {
                unorm
            } else {
                Format::BC5_UNORM_BLOCK
            }
        }

        // HDR / float formats
        TextureFormat::Rgb9e5Ufloat
        | TextureFormat::Rg11b10Ufloat
        | TextureFormat::R16Float
        | TextureFormat::Rg16Float
        | TextureFormat::Rgba16Float => {
            if let Some((_, _, hdr)) = astc_block {
                hdr
            } else {
                Format::BC6H_UFLOAT_BLOCK
            }
        }

        // 4-channel LDR
        TextureFormat::Rgba8Unorm | TextureFormat::Bgra8Unorm | TextureFormat::Rgb10a2Unorm => {
            if let Some((unorm, _, _)) = astc_block {
                unorm
            } else {
                Format::BC7_UNORM_BLOCK
            }
        }
        TextureFormat::Rgba8UnormSrgb | TextureFormat::Bgra8UnormSrgb => {
            if let Some((_, srgb, _)) = astc_block {
                srgb
            } else {
                Format::BC7_SRGB_BLOCK
            }
        }

        // Already compressed -> pass through as compressed
        TextureFormat::Bc1RgbaUnorm
        | TextureFormat::Bc1RgbaUnormSrgb
        | TextureFormat::Bc2RgbaUnorm
        | TextureFormat::Bc2RgbaUnormSrgb
        | TextureFormat::Bc3RgbaUnorm
        | TextureFormat::Bc3RgbaUnormSrgb
        | TextureFormat::Bc4RUnorm
        | TextureFormat::Bc4RSnorm
        | TextureFormat::Bc5RgUnorm
        | TextureFormat::Bc5RgSnorm
        | TextureFormat::Bc6hRgbUfloat
        | TextureFormat::Bc6hRgbFloat
        | TextureFormat::Bc7RgbaUnorm
        | TextureFormat::Bc7RgbaUnormSrgb
        | TextureFormat::Etc2Rgb8Unorm
        | TextureFormat::Etc2Rgb8UnormSrgb
        | TextureFormat::Etc2Rgb8A1Unorm
        | TextureFormat::Etc2Rgb8A1UnormSrgb
        | TextureFormat::Etc2Rgba8Unorm
        | TextureFormat::Etc2Rgba8UnormSrgb
        | TextureFormat::EacR11Unorm
        | TextureFormat::EacR11Snorm
        | TextureFormat::EacRg11Unorm
        | TextureFormat::EacRg11Snorm
        | TextureFormat::Astc { .. } => map_to_ctt_texture_format(input)?,

        // Integer, high-precision, and float formats -> pass through uncompressed
        TextureFormat::R8Uint
        | TextureFormat::R8Sint
        | TextureFormat::R16Uint
        | TextureFormat::R16Sint
        | TextureFormat::R16Unorm
        | TextureFormat::R16Snorm
        | TextureFormat::R32Uint
        | TextureFormat::R32Sint
        | TextureFormat::R32Float
        | TextureFormat::R64Uint
        | TextureFormat::Rg8Uint
        | TextureFormat::Rg8Sint
        | TextureFormat::Rg16Uint
        | TextureFormat::Rg16Sint
        | TextureFormat::Rg16Unorm
        | TextureFormat::Rg16Snorm
        | TextureFormat::Rg32Uint
        | TextureFormat::Rg32Sint
        | TextureFormat::Rg32Float
        | TextureFormat::Rgba8Uint
        | TextureFormat::Rgba8Sint
        | TextureFormat::Rgba8Snorm
        | TextureFormat::Rgba16Uint
        | TextureFormat::Rgba16Sint
        | TextureFormat::Rgba16Unorm
        | TextureFormat::Rgba16Snorm
        | TextureFormat::Rgba32Uint
        | TextureFormat::Rgba32Sint
        | TextureFormat::Rgba32Float
        | TextureFormat::Rgb10a2Uint => {
            return Ok(ctt::TargetFormat::Uncompressed(map_to_ctt_texture_format(
                input,
            )?));
        }

        // Depth/stencil and video formats cannot be compressed
        TextureFormat::Stencil8
        | TextureFormat::Depth16Unorm
        | TextureFormat::Depth24Plus
        | TextureFormat::Depth24PlusStencil8
        | TextureFormat::Depth32Float
        | TextureFormat::Depth32FloatStencil8
        | TextureFormat::NV12
        | TextureFormat::P010 => {
            return Err(CompressedImageSaverError::UnsupportedFormat(input));
        }
    };

    Ok(ctt::TargetFormat::Compressed {
        encoder_name: None,
        format,
    })
}

#[cfg(feature = "compressed_image_saver")]
fn map_to_ctt_texture_format(
    input: TextureFormat,
) -> Result<ctt::Format, CompressedImageSaverError> {
    use ctt::Format;
    use wgpu_types::{AstcBlock, AstcChannel};

    Ok(match input {
        TextureFormat::R8Unorm => Format::R8_UNORM,
        TextureFormat::R8Snorm => Format::R8_SNORM,
        TextureFormat::R8Uint => Format::R8_UINT,
        TextureFormat::R8Sint => Format::R8_SINT,
        TextureFormat::R16Uint => Format::R16_UINT,
        TextureFormat::R16Sint => Format::R16_SINT,
        TextureFormat::R16Unorm => Format::R16_UNORM,
        TextureFormat::R16Snorm => Format::R16_SNORM,
        TextureFormat::R16Float => Format::R16_SFLOAT,
        TextureFormat::Rg8Unorm => Format::R8G8_UNORM,
        TextureFormat::Rg8Snorm => Format::R8G8_SNORM,
        TextureFormat::Rg8Uint => Format::R8G8_UINT,
        TextureFormat::Rg8Sint => Format::R8G8_SINT,
        TextureFormat::R32Uint => Format::R32_UINT,
        TextureFormat::R32Sint => Format::R32_SINT,
        TextureFormat::R32Float => Format::R32_SFLOAT,
        TextureFormat::Rg16Uint => Format::R16G16_UINT,
        TextureFormat::Rg16Sint => Format::R16G16_SINT,
        TextureFormat::Rg16Unorm => Format::R16G16_UNORM,
        TextureFormat::Rg16Snorm => Format::R16G16_SNORM,
        TextureFormat::Rg16Float => Format::R16G16_SFLOAT,
        TextureFormat::Rgba8Unorm => Format::R8G8B8A8_UNORM,
        TextureFormat::Rgba8UnormSrgb => Format::R8G8B8A8_SRGB,
        TextureFormat::Rgba8Snorm => Format::R8G8B8A8_SNORM,
        TextureFormat::Rgba8Uint => Format::R8G8B8A8_UINT,
        TextureFormat::Rgba8Sint => Format::R8G8B8A8_SINT,
        TextureFormat::Bgra8Unorm => Format::B8G8R8A8_UNORM,
        TextureFormat::Bgra8UnormSrgb => Format::B8G8R8A8_SRGB,
        TextureFormat::Rgb9e5Ufloat => Format::E5B9G9R9_UFLOAT_PACK32,
        TextureFormat::Rgb10a2Uint => Format::A2B10G10R10_UINT_PACK32,
        TextureFormat::Rgb10a2Unorm => Format::A2B10G10R10_UNORM_PACK32,
        TextureFormat::Rg11b10Ufloat => Format::B10G11R11_UFLOAT_PACK32,
        TextureFormat::R64Uint => Format::R64_UINT,
        TextureFormat::Rg32Uint => Format::R32G32_UINT,
        TextureFormat::Rg32Sint => Format::R32G32_SINT,
        TextureFormat::Rg32Float => Format::R32G32_SFLOAT,
        TextureFormat::Rgba16Uint => Format::R16G16B16A16_UINT,
        TextureFormat::Rgba16Sint => Format::R16G16B16A16_SINT,
        TextureFormat::Rgba16Unorm => Format::R16G16B16A16_UNORM,
        TextureFormat::Rgba16Snorm => Format::R16G16B16A16_SNORM,
        TextureFormat::Rgba16Float => Format::R16G16B16A16_SFLOAT,
        TextureFormat::Rgba32Uint => Format::R32G32B32A32_UINT,
        TextureFormat::Rgba32Sint => Format::R32G32B32A32_SINT,
        TextureFormat::Rgba32Float => Format::R32G32B32A32_SFLOAT,
        TextureFormat::Stencil8 => Format::S8_UINT,
        TextureFormat::Depth16Unorm => Format::D16_UNORM,
        TextureFormat::Depth24Plus => Format::X8_D24_UNORM_PACK32,
        TextureFormat::Depth24PlusStencil8 => Format::D24_UNORM_S8_UINT,
        TextureFormat::Depth32Float => Format::D32_SFLOAT,
        TextureFormat::Depth32FloatStencil8 => Format::D32_SFLOAT_S8_UINT,
        TextureFormat::NV12 | TextureFormat::P010 => {
            return Err(CompressedImageSaverError::UnsupportedFormat(input));
        }
        TextureFormat::Bc1RgbaUnorm => Format::BC1_RGBA_UNORM_BLOCK,
        TextureFormat::Bc1RgbaUnormSrgb => Format::BC1_RGBA_SRGB_BLOCK,
        TextureFormat::Bc2RgbaUnorm => Format::BC2_UNORM_BLOCK,
        TextureFormat::Bc2RgbaUnormSrgb => Format::BC2_SRGB_BLOCK,
        TextureFormat::Bc3RgbaUnorm => Format::BC3_UNORM_BLOCK,
        TextureFormat::Bc3RgbaUnormSrgb => Format::BC3_SRGB_BLOCK,
        TextureFormat::Bc4RUnorm => Format::BC4_UNORM_BLOCK,
        TextureFormat::Bc4RSnorm => Format::BC4_SNORM_BLOCK,
        TextureFormat::Bc5RgUnorm => Format::BC5_UNORM_BLOCK,
        TextureFormat::Bc5RgSnorm => Format::BC5_SNORM_BLOCK,
        TextureFormat::Bc6hRgbUfloat => Format::BC6H_UFLOAT_BLOCK,
        TextureFormat::Bc6hRgbFloat => Format::BC6H_SFLOAT_BLOCK,
        TextureFormat::Bc7RgbaUnorm => Format::BC7_UNORM_BLOCK,
        TextureFormat::Bc7RgbaUnormSrgb => Format::BC7_SRGB_BLOCK,
        TextureFormat::Etc2Rgb8Unorm => Format::ETC2_R8G8B8_UNORM_BLOCK,
        TextureFormat::Etc2Rgb8UnormSrgb => Format::ETC2_R8G8B8_SRGB_BLOCK,
        TextureFormat::Etc2Rgb8A1Unorm => Format::ETC2_R8G8B8A1_UNORM_BLOCK,
        TextureFormat::Etc2Rgb8A1UnormSrgb => Format::ETC2_R8G8B8A1_SRGB_BLOCK,
        TextureFormat::Etc2Rgba8Unorm => Format::ETC2_R8G8B8A8_UNORM_BLOCK,
        TextureFormat::Etc2Rgba8UnormSrgb => Format::ETC2_R8G8B8A8_SRGB_BLOCK,
        TextureFormat::EacR11Unorm => Format::EAC_R11_UNORM_BLOCK,
        TextureFormat::EacR11Snorm => Format::EAC_R11_SNORM_BLOCK,
        TextureFormat::EacRg11Unorm => Format::EAC_R11G11_UNORM_BLOCK,
        TextureFormat::EacRg11Snorm => Format::EAC_R11G11_SNORM_BLOCK,
        TextureFormat::Astc { block, channel } => match (block, channel) {
            (AstcBlock::B4x4, AstcChannel::Unorm) => Format::ASTC_4x4_UNORM_BLOCK,
            (AstcBlock::B4x4, AstcChannel::UnormSrgb) => Format::ASTC_4x4_SRGB_BLOCK,
            (AstcBlock::B4x4, AstcChannel::Hdr) => Format::ASTC_4x4_SFLOAT_BLOCK,
            (AstcBlock::B5x4, AstcChannel::Unorm) => Format::ASTC_5x4_UNORM_BLOCK,
            (AstcBlock::B5x4, AstcChannel::UnormSrgb) => Format::ASTC_5x4_SRGB_BLOCK,
            (AstcBlock::B5x4, AstcChannel::Hdr) => Format::ASTC_5x4_SFLOAT_BLOCK,
            (AstcBlock::B5x5, AstcChannel::Unorm) => Format::ASTC_5x5_UNORM_BLOCK,
            (AstcBlock::B5x5, AstcChannel::UnormSrgb) => Format::ASTC_5x5_SRGB_BLOCK,
            (AstcBlock::B5x5, AstcChannel::Hdr) => Format::ASTC_5x5_SFLOAT_BLOCK,
            (AstcBlock::B6x5, AstcChannel::Unorm) => Format::ASTC_6x5_UNORM_BLOCK,
            (AstcBlock::B6x5, AstcChannel::UnormSrgb) => Format::ASTC_6x5_SRGB_BLOCK,
            (AstcBlock::B6x5, AstcChannel::Hdr) => Format::ASTC_6x5_SFLOAT_BLOCK,
            (AstcBlock::B6x6, AstcChannel::Unorm) => Format::ASTC_6x6_UNORM_BLOCK,
            (AstcBlock::B6x6, AstcChannel::UnormSrgb) => Format::ASTC_6x6_SRGB_BLOCK,
            (AstcBlock::B6x6, AstcChannel::Hdr) => Format::ASTC_6x6_SFLOAT_BLOCK,
            (AstcBlock::B8x5, AstcChannel::Unorm) => Format::ASTC_8x5_UNORM_BLOCK,
            (AstcBlock::B8x5, AstcChannel::UnormSrgb) => Format::ASTC_8x5_SRGB_BLOCK,
            (AstcBlock::B8x5, AstcChannel::Hdr) => Format::ASTC_8x5_SFLOAT_BLOCK,
            (AstcBlock::B8x6, AstcChannel::Unorm) => Format::ASTC_8x6_UNORM_BLOCK,
            (AstcBlock::B8x6, AstcChannel::UnormSrgb) => Format::ASTC_8x6_SRGB_BLOCK,
            (AstcBlock::B8x6, AstcChannel::Hdr) => Format::ASTC_8x6_SFLOAT_BLOCK,
            (AstcBlock::B8x8, AstcChannel::Unorm) => Format::ASTC_8x8_UNORM_BLOCK,
            (AstcBlock::B8x8, AstcChannel::UnormSrgb) => Format::ASTC_8x8_SRGB_BLOCK,
            (AstcBlock::B8x8, AstcChannel::Hdr) => Format::ASTC_8x8_SFLOAT_BLOCK,
            (AstcBlock::B10x5, AstcChannel::Unorm) => Format::ASTC_10x5_UNORM_BLOCK,
            (AstcBlock::B10x5, AstcChannel::UnormSrgb) => Format::ASTC_10x5_SRGB_BLOCK,
            (AstcBlock::B10x5, AstcChannel::Hdr) => Format::ASTC_10x5_SFLOAT_BLOCK,
            (AstcBlock::B10x6, AstcChannel::Unorm) => Format::ASTC_10x6_UNORM_BLOCK,
            (AstcBlock::B10x6, AstcChannel::UnormSrgb) => Format::ASTC_10x6_SRGB_BLOCK,
            (AstcBlock::B10x6, AstcChannel::Hdr) => Format::ASTC_10x6_SFLOAT_BLOCK,
            (AstcBlock::B10x8, AstcChannel::Unorm) => Format::ASTC_10x8_UNORM_BLOCK,
            (AstcBlock::B10x8, AstcChannel::UnormSrgb) => Format::ASTC_10x8_SRGB_BLOCK,
            (AstcBlock::B10x8, AstcChannel::Hdr) => Format::ASTC_10x8_SFLOAT_BLOCK,
            (AstcBlock::B10x10, AstcChannel::Unorm) => Format::ASTC_10x10_UNORM_BLOCK,
            (AstcBlock::B10x10, AstcChannel::UnormSrgb) => Format::ASTC_10x10_SRGB_BLOCK,
            (AstcBlock::B10x10, AstcChannel::Hdr) => Format::ASTC_10x10_SFLOAT_BLOCK,
            (AstcBlock::B12x10, AstcChannel::Unorm) => Format::ASTC_12x10_UNORM_BLOCK,
            (AstcBlock::B12x10, AstcChannel::UnormSrgb) => Format::ASTC_12x10_SRGB_BLOCK,
            (AstcBlock::B12x10, AstcChannel::Hdr) => Format::ASTC_12x10_SFLOAT_BLOCK,
            (AstcBlock::B12x12, AstcChannel::Unorm) => Format::ASTC_12x12_UNORM_BLOCK,
            (AstcBlock::B12x12, AstcChannel::UnormSrgb) => Format::ASTC_12x12_SRGB_BLOCK,
            (AstcBlock::B12x12, AstcChannel::Hdr) => Format::ASTC_12x12_SFLOAT_BLOCK,
        },
    })
}
