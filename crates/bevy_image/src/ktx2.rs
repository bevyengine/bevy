use bevy_color::Srgba;
use bevy_utils::default;
use ktx2::{
    ChannelTypeQualifiers, ColorModel, DfdBlockBasic, DfdHeader, Header, SampleInformation,
    SupercompressionScheme,
};
use thiserror::Error;
use wgpu_types::{
    AstcBlock, AstcChannel, Extent3d, Features, TextureDataOrder, TextureDimension, TextureFormat,
    TextureViewDescriptor, TextureViewDimension,
};

use super::{CompressedImageFormats, DataFormat, Image, TextureError};

#[cfg(feature = "basis-universal")]
use basis_universal::{
    DecodeFlags, LowLevelUastcLdr4x4Transcoder, SliceParametersUastc, TranscoderBlockFormat,
};

#[derive(Error, Debug)]
pub enum Ktx2TextureError {
    /// This texture data is not supported and there's no way to transcode it.
    #[error("unsupported ktx2 texture: {0}")]
    Unsupported(String),

    /// The texture data cannot be used as-is; it needs to be transcoded into a format that wgpu can understand.
    ///
    /// Like [`Ktx2TextureError::Unsupported`], but with extra information to help Bevy transcode it.
    #[error("unsupported ktx2 texture: {0:?}")]
    RequiresTranscoding(Ktx2TranscodingHint),

    /// The KTX2 data is malformed and no transcoding should be attempted.
    #[error("invalid ktx2 texture: {0}")]
    Invalid(String),
}

impl From<Ktx2TextureError> for TextureError {
    fn from(val: Ktx2TextureError) -> Self {
        match val {
            Ktx2TextureError::Unsupported(message) => {
                TextureError::UnsupportedTextureFormat(message)
            }
            Ktx2TextureError::Invalid(message) => TextureError::InvalidData(message),
            Ktx2TextureError::RequiresTranscoding(..) => {
                TextureError::TranscodeError("This image requires transcoding".to_string())
            }
        }
    }
}

/// Bevy-specific information on what the underlying KTX2 data format is that needs transcoded for wgpu.
///
/// This enum is non-exhaustive – it is only meant meant to hold formats that Bevy actually knows how to transcode itself.
#[derive(Clone, Copy, Debug)]
pub enum Ktx2TranscodingHint {
    /// Use basis-universal's low-level UASTC LDR 4x4 transcoder to decode the data to a supported format
    UastcLdr4x4 {
        is_srgb: bool,
        data_format: DataFormat,
    },
    /// Conversion from sRGB to Linear is needed (to end up as [`TextureFormat::R8Unorm`])
    R8UnormSrgb,
    /// Conversion from sRGB to Linear is needed (to end up as [`TextureFormat::Rg8Unorm`])
    Rg8UnormSrgb,
    /// This needs an alpha channel added (to end up as [`TextureFormat::Rgba8Unorm`] or [`TextureFormat::Rgba8UnormSrgb`])
    Rgb8 { is_srgb: bool },
    /// This needs an alpha channel added (to end up as [`TextureFormat::Rgba32Float`])
    Rgb32Float,
    /// This needs an alpha channel added (to end up as [`TextureFormat::Rgba32Sint`])
    Rgb32Sint,
    /// This needs an alpha channel added (to end up as [`TextureFormat::Rgba32Uint`])
    Rgb32Uint,
}

/// Decodes/transcodes a KTX2 image.
///
/// If the `"basis-universal"` feature is enabled, it will fall back to using [basis-universal](https://crates.io/crates/basis-universal)
/// for special cases (like ETC1S/BasisLZ). If you have an image with special needs that Bevy's Rust
/// frontend doesn't handle well (or you know will always be kicked out to `basis-universal`, like
/// a ETC1S/BasisLZ image), feel free to use [`crate::ktx2_buffer_to_image_using_basisu`] directly.
pub fn ktx2_buffer_to_image(
    buffer: &[u8],
    supported_compressed_formats: CompressedImageFormats,
) -> Result<Image, TextureError> {
    let ktx2 = ktx2::Reader::new(buffer)
        .map_err(|err| TextureError::InvalidData(format!("Failed to parse ktx2 file: {err:?}")))?;
    let Header {
        pixel_width: width,
        pixel_height: height,
        pixel_depth: depth,
        layer_count,
        face_count,
        level_count,
        supercompression_scheme,
        ..
    } = ktx2.header();
    let layer_count = layer_count.max(1);
    let face_count = face_count.max(1);
    let depth: u32 = depth.max(1);
    let texture_format: Result<TextureFormat, Ktx2TextureError> = ktx2_get_texture_format(&ktx2);
    let mut use_basis_universal_decoder = false;

    // BasisLZ supercompression (ETC1S) requires transcoding via basis-universal
    if let Some(supercompression_scheme) = supercompression_scheme {
        if supercompression_scheme == SupercompressionScheme::BasisLZ {
            use_basis_universal_decoder = true;
        }
    }

    // If the system doesn't support the compressed format, try to decompress the data using basis-universal
    // if it is not UASTC. UASTC is handled using `LowLevelUastcLdr4x4Transcoder` further below, so there's
    // no need to handle it here. Currently, basis-universal only supports ETC and UASTC decompression.
    if let Ok(texture_format) = texture_format {
        if texture_format.is_compressed() && !supported_compressed_formats.supports(texture_format)
        {
            let is_etc2 = texture_format.required_features() == Features::TEXTURE_COMPRESSION_ETC2;
            use_basis_universal_decoder = is_etc2;
        }
    }

    if use_basis_universal_decoder {
        #[cfg(feature = "basis-universal")]
        {
            return crate::ktx2_using_basisu::ktx2_buffer_to_image_using_basisu(
                ktx2.data(),
                supported_compressed_formats,
            );
        }
        #[cfg(not(feature = "basis-universal"))]
        return Err(TextureError::UnsupportedTextureFormat(
            "This image requires Bevy to be compiled with the 'basis-universal' feature"
                .to_string(),
        ));
    }

    // Handle supercompression
    #[cfg(not(any(feature = "flate2", feature = "zstd_native", feature = "zstd_rust")))]
    if let Some(supercompression_scheme) = supercompression_scheme {
        return Err(TextureError::SuperDecompressionError(format!(
            "Unsupported supercompression scheme: {supercompression_scheme:?}",
        )));
    }

    // Decompressed/transcoded levels. This starts out as the raw data from the KTX, but if we decompress
    // supercompression or transcode a level, we update the reference to point to the new level data.
    let mut levels = ktx2.levels().map(|level| level.data).collect::<Vec<_>>();

    // Level data that has been decompressed and/or decoded (if necessary).
    let mut scratch_levels = Vec::new();

    #[cfg(any(feature = "flate2", feature = "zstd_native", feature = "zstd_rust"))]
    if let Some(supercompression_scheme) = supercompression_scheme {
        scratch_levels.reserve_exact(levels.len());
        for (_level, _level_data) in levels.iter_mut().enumerate() {
            let decompressed = match supercompression_scheme {
                #[cfg(feature = "basis-universal")]
                SupercompressionScheme::BasisLZ => unreachable!(),
                #[cfg(feature = "flate2")]
                SupercompressionScheme::ZLIB => {
                    use std::io::Read;
                    let mut decoder = flate2::bufread::ZlibDecoder::new(_level_data);
                    let mut decompressed = Vec::new();
                    decoder.read_to_end(&mut decompressed).map_err(|err| {
                            TextureError::SuperDecompressionError(format!(
                                "Failed to decompress {supercompression_scheme:?} for mip {_level}: {err:?}",
                            ))
                        })?;

                    decompressed
                }
                #[cfg(feature = "zstd_native")]
                SupercompressionScheme::Zstandard => {
                    use zstd::decode_all;
                    decode_all(_level_data).map_err(|err| {
                        TextureError::SuperDecompressionError(format!(
                            "Failed to decompress {supercompression_scheme:?} for mip {_level} using zstd: {err:?}",
                        ))
                    })?
                }
                #[cfg(all(feature = "zstd_rust", not(feature = "zstd_native")))]
                SupercompressionScheme::Zstandard => {
                    use std::io::Read;
                    let mut cursor = std::io::Cursor::new(_level_data);
                    let mut decoder = ruzstd::decoding::StreamingDecoder::new(&mut cursor)
                        .map_err(|err| TextureError::SuperDecompressionError(err.to_string()))?;
                    let mut decompressed = Vec::new();
                    decoder.read_to_end(&mut decompressed).map_err(|err| {
                            TextureError::SuperDecompressionError(format!(
                                "Failed to decompress {supercompression_scheme:?} for mip {_level} using ruzstd: {err:?}",
                            ))
                        })?;
                    decompressed
                }
                _ => {
                    return Err(TextureError::SuperDecompressionError(format!(
                        "Unsupported supercompression scheme: {supercompression_scheme:?}",
                    )));
                }
            };

            scratch_levels.push(decompressed);
        }
    }

    // Transcode if needed and supported
    let texture_format = texture_format.or_else(|error| match error {
        Ktx2TextureError::RequiresTranscoding(transcoding_hint) => {
            if scratch_levels.is_empty() {
                scratch_levels.reserve_exact(levels.len());
                scratch_levels.extend(levels.iter().map(|level| level.to_vec()));
            }
            let texture_format = match transcoding_hint {
                Ktx2TranscodingHint::R8UnormSrgb => {
                    for level_data in scratch_levels.iter_mut() {
                        for byte in level_data.iter_mut() {
                            *byte = Srgba::gamma_function(((*byte) as f32 / 255.) * 255.).floor() as u8;
                        }
                    }
                    TextureFormat::R8Unorm
                }
                Ktx2TranscodingHint::Rg8UnormSrgb => {
                    for level_data in scratch_levels.iter_mut() {
                        for byte in level_data.iter_mut() {
                            *byte = Srgba::gamma_function(((*byte) as f32 / 255.) * 255.).floor() as u8;
                        }
                    }
                    TextureFormat::Rg8Unorm
                }
                Ktx2TranscodingHint::Rgb8 { is_srgb } => {
                    // Add an alpha channel
                    for level_data in scratch_levels.iter_mut() {
                        let pixel_count = level_data.len() / 3;
                        let mut rgba = vec![255u8; pixel_count * 4];
                        let mut offset = 0;
                        for i in 0..pixel_count {
                            rgba[offset] = level_data[i * 3];
                            offset += 1;
                            rgba[offset] = level_data[i * 3 + 1];
                            offset += 1;
                            rgba[offset] = level_data[i * 3 + 2];
                            offset += 2;
                        }
                        *level_data = rgba;
                    }

                    if is_srgb {
                        TextureFormat::Rgba8UnormSrgb
                    } else {
                        TextureFormat::Rgba8Unorm
                    }
                }
                Ktx2TranscodingHint::Rgb32Float => {
                    // Add an alpha channel
                    let alpha = [0u8, 0u8, 128u8, 63u8]; // 1.0f32
                    for level_data in scratch_levels.iter_mut() {
                        let mut rgba = Vec::with_capacity(level_data.len() / 3 * 4);
                        for rgb in level_data.chunks_exact(3 * 4) {
                            rgba.extend_from_slice(rgb);
                            rgba.extend_from_slice(&alpha);
                        }
                        *level_data = rgba;
                    }

                    TextureFormat::Rgba32Float
                }
                Ktx2TranscodingHint::Rgb32Sint | Ktx2TranscodingHint::Rgb32Uint => {
                    // Add an alpha channel
                    let zero = [0u8, 0u8, 0u8, 0u8]; // 0
                    for level_data in scratch_levels.iter_mut() {
                        let mut rgba = Vec::with_capacity(level_data.len() / 3 * 4);
                        for rgb in level_data.chunks_exact(3 * 4) {
                            rgba.extend_from_slice(rgb);
                            rgba.extend_from_slice(&zero);
                        }
                        *level_data = rgba;
                    }
                    if matches!(transcoding_hint, Ktx2TranscodingHint::Rgb32Sint) {
                        TextureFormat::Rgba32Sint
                    } else {
                        TextureFormat::Rgba32Uint
                    }
                }
                #[cfg(not(feature = "basis-universal"))]
                Ktx2TranscodingHint::UastcLdr4x4 { .. }  => {
                    return Err(TextureError::UnsupportedTextureFormat("UASTC texture decompression requires the 'basis-universal' feature".to_string()));
                }
                #[cfg(feature = "basis-universal")]
                Ktx2TranscodingHint::UastcLdr4x4 { data_format, is_srgb }  => {
                    let (transcode_block_format, texture_format) =
                        get_uastc_ldr4x4_transcoded_formats(supported_compressed_formats, data_format, is_srgb);

                    let input_bpbp = TranscoderBlockFormat::ASTC_4x4.bytes_per_block_or_pixel() as usize;
                    let output_bpbp = transcode_block_format.bytes_per_block_or_pixel() as usize;

                    // Ensure wgpu and basis-universal are reporting the same output block size for the target format
                    debug_assert_eq!(output_bpbp, texture_format.block_copy_size(None).unwrap() as usize);

                    // Determine if the transcoded size of the output occupies the same amount of memory
                    let output_is_same_shape = texture_format.is_compressed() && input_bpbp == output_bpbp;

                    let transcoder = LowLevelUastcLdr4x4Transcoder::new();
                    for (level, level_data) in scratch_levels.iter_mut().enumerate() {
                        let (level_width, level_height) = (
                            (width >> level as u32).max(1),
                            (height >> level as u32).max(1),
                        );

                        // Number of blocks in each dimension (of the input, not the output)
                        let (num_blocks_x, num_blocks_y) = (
                            level_width.div_ceil(4).max(1),
                            level_height.div_ceil(4).max(1),
                        );

                        let level_bytes = (num_blocks_x * num_blocks_y) as usize * input_bpbp;
                        let slice_parameters = SliceParametersUastc {
                            num_blocks_x,
                            num_blocks_y,
                            has_alpha: false,
                            original_width: level_width,
                            original_height: level_height,
                        };

                        let mut offset = 0;
                        let mut buffered_slices = Vec::new();
                        for _layer in 0..layer_count {
                            for _face in 0..face_count {
                                let input_slice = &mut level_data[offset..(offset + level_bytes)];
                                let transcoded_slice = transcoder
                                    .transcode_slice(
                                        input_slice.as_ref(),
                                        slice_parameters,
                                        DecodeFlags::HIGH_QUALITY,
                                        transcode_block_format,
                                    )
                                    .map_err(|error| {
                                        TextureError::SuperDecompressionError(format!(
                                            "Failed to transcode mip level {level} from UASTC to {transcode_block_format:?}: {error:?}",
                                        ))
                                    })?;

                                if output_is_same_shape {
                                    // Overwrite the slice in the scratch level because it's the same size (no need to buffer)
                                    input_slice.copy_from_slice(transcoded_slice.as_slice());
                                } else {
                                    buffered_slices.push(transcoded_slice);
                                }

                                offset += level_bytes;
                            }
                        }

                        if !output_is_same_shape {
                            // Here we resize the existing scratch level buffer to the new size and write into it.
                            let new_size = buffered_slices.iter().map(Vec::len).sum();
                            let old_size = level_data.len();
                            if new_size > old_size {
                                level_data.reserve_exact(new_size - old_size);
                                // SAFETY: New length is equal to the capacity reserved in the line above. All new elements are initialized below.
                                #[expect(unsafe_code, reason = "We could call resize(), but that performs element-by-element initialization, which is redundant with the copy_from_slice initialization below (thereby less performant)")]
                                unsafe { level_data.set_len(new_size); }
                            } else {
                                level_data.resize(new_size, 0);
                            }

                            let mut offset = 0;
                            for slice in buffered_slices.into_iter() {
                                let slice_length = slice.len();
                                level_data[offset..slice_length].copy_from_slice(slice.as_slice());
                                offset += slice_length;
                            }
                        }
                    }
                    texture_format
                },
            };
            Ok::<TextureFormat, TextureError>(texture_format)
        }
        _ => Err(error.into()),
    })?;

    if !supported_compressed_formats.supports(texture_format) {
        return Err(TextureError::UnsupportedTextureFormat(format!(
            "Format not supported by this GPU: {texture_format:?}",
        )));
    }

    // Collect all level data into a contiguous buffer
    let contiguous_level_data = if !scratch_levels.is_empty() {
        let final_buffer_size = scratch_levels.iter().map(Vec::len).sum();
        let mut contiguous_level_data = Vec::with_capacity(final_buffer_size);
        for mut level in scratch_levels.into_iter() {
            contiguous_level_data.append(&mut level);
        }
        contiguous_level_data
    } else {
        let final_buffer_size = levels.iter().map(|level| level.len()).sum();
        let mut contiguous_level_data = Vec::with_capacity(final_buffer_size);
        for level in levels.into_iter() {
            contiguous_level_data.extend_from_slice(level);
        }
        contiguous_level_data
    };

    // Assign the data and fill in the rest of the metadata now the possible
    // error cases have been handled
    let mut image = Image::default();
    image.texture_descriptor.format = texture_format;
    image.data = Some(contiguous_level_data);
    image.data_order = TextureDataOrder::MipMajor;
    image.texture_descriptor.size = Extent3d {
        width,
        height,
        depth_or_array_layers: if layer_count > 1 || face_count > 1 {
            layer_count * face_count
        } else {
            depth
        }
        .max(1),
    }
    .physical_size(texture_format);
    image.texture_descriptor.mip_level_count = level_count;
    image.texture_descriptor.dimension = if depth > 1 {
        TextureDimension::D3
    } else if image.is_compressed() || height > 1 {
        TextureDimension::D2
    } else {
        TextureDimension::D1
    };
    let mut dimension = None;
    if face_count == 6 {
        dimension = Some(if layer_count > 1 {
            TextureViewDimension::CubeArray
        } else {
            TextureViewDimension::Cube
        });
    } else if layer_count > 1 {
        dimension = Some(TextureViewDimension::D2Array);
    } else if depth > 1 {
        dimension = Some(TextureViewDimension::D3);
    }
    if dimension.is_some() {
        image.texture_view_descriptor = Some(TextureViewDescriptor {
            dimension,
            ..default()
        });
    }
    Ok(image)
}

#[cfg(feature = "basis-universal")]
pub fn get_uastc_ldr4x4_transcoded_formats(
    supported_compressed_formats: CompressedImageFormats,
    data_format: DataFormat,
    is_srgb: bool,
) -> (TranscoderBlockFormat, TextureFormat) {
    match data_format {
        DataFormat::Rrr => {
            if supported_compressed_formats.contains(CompressedImageFormats::BC) {
                (TranscoderBlockFormat::BC4, TextureFormat::Bc4RUnorm)
            } else if supported_compressed_formats.contains(CompressedImageFormats::ETC2) {
                (
                    TranscoderBlockFormat::ETC2_EAC_R11,
                    TextureFormat::EacR11Unorm,
                )
            } else {
                (TranscoderBlockFormat::RGBA32, TextureFormat::R8Unorm)
            }
        }
        DataFormat::Rrrg | DataFormat::Rg => {
            if supported_compressed_formats.contains(CompressedImageFormats::BC) {
                (TranscoderBlockFormat::BC5, TextureFormat::Bc5RgUnorm)
            } else if supported_compressed_formats.contains(CompressedImageFormats::ETC2) {
                (
                    TranscoderBlockFormat::ETC2_EAC_RG11,
                    TextureFormat::EacRg11Unorm,
                )
            } else {
                (TranscoderBlockFormat::RGBA32, TextureFormat::Rg8Unorm)
            }
        }
        // NOTE: Rgba16Float should be transcoded to BC6H/ASTC_HDR. Neither are supported by
        // basis-universal, nor is ASTC_HDR supported by wgpu
        DataFormat::Rgb | DataFormat::Rgba => {
            // NOTE: UASTC can be losslessly transcoded to ASTC4x4 and ASTC uses the same
            // space as BC7 (128-bits per 4x4 texel block) so prefer ASTC over BC for
            // transcoding speed and quality.
            if supported_compressed_formats.contains(CompressedImageFormats::ASTC_LDR) {
                (
                    TranscoderBlockFormat::ASTC_4x4,
                    TextureFormat::Astc {
                        block: AstcBlock::B4x4,
                        channel: if is_srgb {
                            AstcChannel::UnormSrgb
                        } else {
                            AstcChannel::Unorm
                        },
                    },
                )
            } else if supported_compressed_formats.contains(CompressedImageFormats::BC) {
                (
                    TranscoderBlockFormat::BC7,
                    if is_srgb {
                        TextureFormat::Bc7RgbaUnormSrgb
                    } else {
                        TextureFormat::Bc7RgbaUnorm
                    },
                )
            } else if supported_compressed_formats.contains(CompressedImageFormats::ETC2) {
                (
                    TranscoderBlockFormat::ETC2_RGBA,
                    if is_srgb {
                        TextureFormat::Etc2Rgba8UnormSrgb
                    } else {
                        TextureFormat::Etc2Rgba8Unorm
                    },
                )
            } else {
                (
                    TranscoderBlockFormat::RGBA32,
                    if is_srgb {
                        TextureFormat::Rgba8UnormSrgb
                    } else {
                        TextureFormat::Rgba8Unorm
                    },
                )
            }
        }
    }
}

pub fn ktx2_get_texture_format<Data: AsRef<[u8]>>(
    ktx2: &ktx2::Reader<Data>,
) -> Result<TextureFormat, Ktx2TextureError> {
    if let Some(format) = ktx2.header().format {
        return ktx2_format_to_texture_format(format);
    }

    for dfd_block in ktx2.dfd_blocks() {
        if dfd_block.header == DfdHeader::BASIC {
            let basic_block = DfdBlockBasic::parse(dfd_block.data).map_err(|err| {
                Ktx2TextureError::Invalid(format!("Unable to parse Basic DFD Block Header {err:?}"))
            })?;

            let sample_information = basic_block.sample_information().collect::<Vec<_>>();
            return ktx2_dfd_to_texture_format(&basic_block, &sample_information);
        }
    }

    Err(Ktx2TextureError::Unsupported(
        "Unable to detect KTX2 data format".to_string(),
    ))
}

enum DataType {
    Unorm,
    UnormSrgb,
    Snorm,
    Float,
    Uint,
    Sint,
}

/// This can be obtained from `core::mem::transmute::<f32, u32>(1.0f32)`. It is used for identifying
/// normalized sample types as in Unorm or Snorm.
const F32_1_AS_U32: u32 = 1065353216;

fn sample_information_to_data_type(
    sample: &SampleInformation,
    is_srgb: bool,
) -> Result<DataType, Ktx2TextureError> {
    // Exponent flag not supported
    if sample
        .channel_type_qualifiers
        .contains(ChannelTypeQualifiers::EXPONENT)
    {
        return Err(Ktx2TextureError::Unsupported(
            "Unsupported KTX2 channel type qualifier: exponent".to_string(),
        ));
    }
    Ok(
        if sample
            .channel_type_qualifiers
            .contains(ChannelTypeQualifiers::FLOAT)
        {
            // If lower bound of range is 0 then unorm, else if upper bound is 1.0f32 as u32
            if sample
                .channel_type_qualifiers
                .contains(ChannelTypeQualifiers::SIGNED)
            {
                if sample.upper == F32_1_AS_U32 {
                    DataType::Snorm
                } else {
                    DataType::Float
                }
            } else if is_srgb {
                DataType::UnormSrgb
            } else {
                DataType::Unorm
            }
        } else if sample
            .channel_type_qualifiers
            .contains(ChannelTypeQualifiers::SIGNED)
        {
            DataType::Sint
        } else {
            DataType::Uint
        },
    )
}

pub fn ktx2_dfd_to_texture_format(
    data_format_descriptor: &DfdBlockBasic,
    sample_information: &[SampleInformation],
) -> Result<TextureFormat, Ktx2TextureError> {
    let is_srgb =
        data_format_descriptor.header.transfer_function == Some(ktx2::TransferFunction::SRGB);

    Ok(match data_format_descriptor.header.color_model {
        Some(ColorModel::RGBSDA) => {
            match sample_information.len() {
                1 => {
                    // Only red channel allowed
                    if sample_information[0].channel_type != 0 {
                        return Err(Ktx2TextureError::Unsupported(
                            "Only red-component single-component KTX2 RGBSDA formats supported"
                                .to_string(),
                        ));
                    }

                    let sample = &sample_information[0];
                    let data_type = sample_information_to_data_type(sample, false)?;
                    match sample.bit_length.get() {
                        8 => match data_type {
                            DataType::Unorm => TextureFormat::R8Unorm,
                            DataType::UnormSrgb => {
                                return Err(Ktx2TextureError::Unsupported(
                                    "UnormSrgb not supported for R8".to_string(),
                                ));
                            }
                            DataType::Snorm => TextureFormat::R8Snorm,
                            DataType::Float => {
                                return Err(Ktx2TextureError::Unsupported(
                                    "Float not supported for R8".to_string(),
                                ));
                            }
                            DataType::Uint => TextureFormat::R8Uint,
                            DataType::Sint => TextureFormat::R8Sint,
                        },
                        16 => match data_type {
                            DataType::Unorm => TextureFormat::R16Unorm,
                            DataType::UnormSrgb => {
                                return Err(Ktx2TextureError::Unsupported(
                                    "UnormSrgb not supported for R16".to_string(),
                                ));
                            }
                            DataType::Snorm => TextureFormat::R16Snorm,
                            DataType::Float => TextureFormat::R16Float,
                            DataType::Uint => TextureFormat::R16Uint,
                            DataType::Sint => TextureFormat::R16Sint,
                        },
                        32 => match data_type {
                            DataType::Unorm => {
                                return Err(Ktx2TextureError::Unsupported(
                                    "Unorm not supported for R32".to_string(),
                                ));
                            }
                            DataType::UnormSrgb => {
                                return Err(Ktx2TextureError::Unsupported(
                                    "UnormSrgb not supported for R32".to_string(),
                                ));
                            }
                            DataType::Snorm => {
                                return Err(Ktx2TextureError::Unsupported(
                                    "Snorm not supported for R32".to_string(),
                                ));
                            }
                            DataType::Float => TextureFormat::R32Float,
                            DataType::Uint => TextureFormat::R32Uint,
                            DataType::Sint => TextureFormat::R32Sint,
                        },
                        v => {
                            return Err(Ktx2TextureError::Unsupported(format!(
                                "Unsupported sample bit length for RGBSDA 1-channel format: {v}",
                            )));
                        }
                    }
                }
                2 => {
                    // Only red and green channels allowed
                    if sample_information[0].channel_type != 0
                        || sample_information[1].channel_type != 1
                    {
                        return Err(Ktx2TextureError::Unsupported(
                            "Only red-green-component two-component KTX2 RGBSDA formats supported"
                                .to_string(),
                        ));
                    }
                    // Only same bit length for all channels
                    assert_eq!(
                        sample_information[0].bit_length,
                        sample_information[1].bit_length
                    );
                    // Only same channel type qualifiers for all channels
                    assert_eq!(
                        sample_information[0].channel_type_qualifiers,
                        sample_information[1].channel_type_qualifiers
                    );
                    // Only same sample range for all channels
                    assert_eq!(sample_information[0].lower, sample_information[1].lower);
                    assert_eq!(sample_information[0].upper, sample_information[1].upper);

                    let sample = &sample_information[0];
                    let data_type = sample_information_to_data_type(sample, false)?;
                    match sample.bit_length.get() {
                        8 => match data_type {
                            DataType::Unorm => TextureFormat::Rg8Unorm,
                            DataType::UnormSrgb => {
                                return Err(Ktx2TextureError::Unsupported(
                                    "UnormSrgb not supported for Rg8".to_string(),
                                ));
                            }
                            DataType::Snorm => TextureFormat::Rg8Snorm,
                            DataType::Float => {
                                return Err(Ktx2TextureError::Unsupported(
                                    "Float not supported for Rg8".to_string(),
                                ));
                            }
                            DataType::Uint => TextureFormat::Rg8Uint,
                            DataType::Sint => TextureFormat::Rg8Sint,
                        },
                        16 => match data_type {
                            DataType::Unorm => TextureFormat::Rg16Unorm,
                            DataType::UnormSrgb => {
                                return Err(Ktx2TextureError::Unsupported(
                                    "UnormSrgb not supported for Rg16".to_string(),
                                ));
                            }
                            DataType::Snorm => TextureFormat::Rg16Snorm,
                            DataType::Float => TextureFormat::Rg16Float,
                            DataType::Uint => TextureFormat::Rg16Uint,
                            DataType::Sint => TextureFormat::Rg16Sint,
                        },
                        32 => match data_type {
                            DataType::Unorm => {
                                return Err(Ktx2TextureError::Unsupported(
                                    "Unorm not supported for Rg32".to_string(),
                                ));
                            }
                            DataType::UnormSrgb => {
                                return Err(Ktx2TextureError::Unsupported(
                                    "UnormSrgb not supported for Rg32".to_string(),
                                ));
                            }
                            DataType::Snorm => {
                                return Err(Ktx2TextureError::Unsupported(
                                    "Snorm not supported for Rg32".to_string(),
                                ));
                            }
                            DataType::Float => TextureFormat::Rg32Float,
                            DataType::Uint => TextureFormat::Rg32Uint,
                            DataType::Sint => TextureFormat::Rg32Sint,
                        },
                        v => {
                            return Err(Ktx2TextureError::Unsupported(format!(
                                "Unsupported sample bit length for RGBSDA 2-channel format: {v}",
                            )));
                        }
                    }
                }
                3 => {
                    if sample_information[0].channel_type == 0
                        && sample_information[0].bit_length.get() == 11
                        && sample_information[1].channel_type == 1
                        && sample_information[1].bit_length.get() == 11
                        && sample_information[2].channel_type == 2
                        && sample_information[2].bit_length.get() == 10
                    {
                        TextureFormat::Rg11b10Ufloat
                    } else if sample_information[0].channel_type == 0
                        && sample_information[0].bit_length.get() == 9
                        && sample_information[1].channel_type == 1
                        && sample_information[1].bit_length.get() == 9
                        && sample_information[2].channel_type == 2
                        && sample_information[2].bit_length.get() == 9
                    {
                        TextureFormat::Rgb9e5Ufloat
                    } else if sample_information[0].channel_type == 0
                        && sample_information[0].bit_length.get() == 8
                        && sample_information[1].channel_type == 1
                        && sample_information[1].bit_length.get() == 8
                        && sample_information[2].channel_type == 2
                        && sample_information[2].bit_length.get() == 8
                    {
                        return Err(Ktx2TextureError::RequiresTranscoding(
                            Ktx2TranscodingHint::Rgb8 { is_srgb },
                        ));
                    } else {
                        return Err(Ktx2TextureError::Unsupported(
                            "3-component formats not supported".to_string(),
                        ));
                    }
                }
                4 => {
                    // Only RGBA or BGRA channels allowed
                    let is_rgba = sample_information[0].channel_type == 0;
                    assert!(
                        sample_information[0].channel_type == 0
                            || sample_information[0].channel_type == 2
                    );
                    assert_eq!(sample_information[1].channel_type, 1);
                    assert_eq!(
                        sample_information[2].channel_type,
                        if is_rgba { 2 } else { 0 }
                    );
                    assert_eq!(sample_information[3].channel_type, 15);

                    // Handle one special packed format
                    if sample_information[0].bit_length.get() == 10
                        && sample_information[1].bit_length.get() == 10
                        && sample_information[2].bit_length.get() == 10
                        && sample_information[3].bit_length.get() == 2
                    {
                        return Ok(TextureFormat::Rgb10a2Unorm);
                    }

                    // Only same bit length for all channels
                    assert!(
                        sample_information[0].bit_length == sample_information[1].bit_length
                            && sample_information[0].bit_length == sample_information[2].bit_length
                            && sample_information[0].bit_length == sample_information[3].bit_length
                    );
                    assert!(
                        sample_information[0].lower == sample_information[1].lower
                            && sample_information[0].lower == sample_information[2].lower
                            && sample_information[0].lower == sample_information[3].lower
                    );
                    assert!(
                        sample_information[0].upper == sample_information[1].upper
                            && sample_information[0].upper == sample_information[2].upper
                            && sample_information[0].upper == sample_information[3].upper
                    );

                    let sample = &sample_information[0];
                    let data_type = sample_information_to_data_type(sample, is_srgb)?;
                    match sample.bit_length.get() {
                        8 => match data_type {
                            DataType::Unorm => {
                                if is_rgba {
                                    TextureFormat::Rgba8Unorm
                                } else {
                                    TextureFormat::Bgra8Unorm
                                }
                            }
                            DataType::UnormSrgb => {
                                if is_rgba {
                                    TextureFormat::Rgba8UnormSrgb
                                } else {
                                    TextureFormat::Bgra8UnormSrgb
                                }
                            }
                            DataType::Snorm => {
                                if is_rgba {
                                    TextureFormat::Rgba8Snorm
                                } else {
                                    return Err(Ktx2TextureError::Unsupported(
                                        "Bgra8 not supported for Snorm".to_string(),
                                    ));
                                }
                            }
                            DataType::Float => {
                                return Err(Ktx2TextureError::Unsupported(
                                    "Float not supported for Rgba8/Bgra8".to_string(),
                                ));
                            }
                            DataType::Uint => {
                                if is_rgba {
                                    // NOTE: This is more about how you want to use the data so
                                    // TextureFormat::Rgba8Uint is incorrect here
                                    if is_srgb {
                                        TextureFormat::Rgba8UnormSrgb
                                    } else {
                                        TextureFormat::Rgba8Unorm
                                    }
                                } else {
                                    return Err(Ktx2TextureError::Unsupported(
                                        "Bgra8 not supported for Uint".to_string(),
                                    ));
                                }
                            }
                            DataType::Sint => {
                                if is_rgba {
                                    // NOTE: This is more about how you want to use the data so
                                    // TextureFormat::Rgba8Sint is incorrect here
                                    TextureFormat::Rgba8Snorm
                                } else {
                                    return Err(Ktx2TextureError::Unsupported(
                                        "Bgra8 not supported for Sint".to_string(),
                                    ));
                                }
                            }
                        },
                        16 => match data_type {
                            DataType::Unorm => {
                                if is_rgba {
                                    TextureFormat::Rgba16Unorm
                                } else {
                                    return Err(Ktx2TextureError::Unsupported(
                                        "Bgra16 not supported for Unorm".to_string(),
                                    ));
                                }
                            }
                            DataType::UnormSrgb => {
                                return Err(Ktx2TextureError::Unsupported(
                                    "UnormSrgb not supported for Rgba16/Bgra16".to_string(),
                                ));
                            }
                            DataType::Snorm => {
                                if is_rgba {
                                    TextureFormat::Rgba16Snorm
                                } else {
                                    return Err(Ktx2TextureError::Unsupported(
                                        "Bgra16 not supported for Snorm".to_string(),
                                    ));
                                }
                            }
                            DataType::Float => {
                                if is_rgba {
                                    TextureFormat::Rgba16Float
                                } else {
                                    return Err(Ktx2TextureError::Unsupported(
                                        "Bgra16 not supported for Float".to_string(),
                                    ));
                                }
                            }
                            DataType::Uint => {
                                if is_rgba {
                                    TextureFormat::Rgba16Uint
                                } else {
                                    return Err(Ktx2TextureError::Unsupported(
                                        "Bgra16 not supported for Uint".to_string(),
                                    ));
                                }
                            }
                            DataType::Sint => {
                                if is_rgba {
                                    TextureFormat::Rgba16Sint
                                } else {
                                    return Err(Ktx2TextureError::Unsupported(
                                        "Bgra16 not supported for Sint".to_string(),
                                    ));
                                }
                            }
                        },
                        32 => match data_type {
                            DataType::Unorm => {
                                return Err(Ktx2TextureError::Unsupported(
                                    "Unorm not supported for Rgba32/Bgra32".to_string(),
                                ));
                            }
                            DataType::UnormSrgb => {
                                return Err(Ktx2TextureError::Unsupported(
                                    "UnormSrgb not supported for Rgba32/Bgra32".to_string(),
                                ));
                            }
                            DataType::Snorm => {
                                return Err(Ktx2TextureError::Unsupported(
                                    "Snorm not supported for Rgba32/Bgra32".to_string(),
                                ));
                            }
                            DataType::Float => {
                                if is_rgba {
                                    TextureFormat::Rgba32Float
                                } else {
                                    return Err(Ktx2TextureError::Unsupported(
                                        "Bgra32 not supported for Float".to_string(),
                                    ));
                                }
                            }
                            DataType::Uint => {
                                if is_rgba {
                                    TextureFormat::Rgba32Uint
                                } else {
                                    return Err(Ktx2TextureError::Unsupported(
                                        "Bgra32 not supported for Uint".to_string(),
                                    ));
                                }
                            }
                            DataType::Sint => {
                                if is_rgba {
                                    TextureFormat::Rgba32Sint
                                } else {
                                    return Err(Ktx2TextureError::Unsupported(
                                        "Bgra32 not supported for Sint".to_string(),
                                    ));
                                }
                            }
                        },
                        v => {
                            return Err(Ktx2TextureError::Unsupported(format!(
                                "Unsupported sample bit length for RGBSDA 4-channel format: {v}",
                            )));
                        }
                    }
                }
                v => {
                    return Err(Ktx2TextureError::Unsupported(format!(
                        "Unsupported channel count for RGBSDA format: {v}",
                    )));
                }
            }
        }
        Some(ColorModel::YUVSDA)
        | Some(ColorModel::YIQSDA)
        | Some(ColorModel::LabSDA)
        | Some(ColorModel::CMYKA)
        | Some(ColorModel::HSVAAng)
        | Some(ColorModel::HSLAAng)
        | Some(ColorModel::HSVAHex)
        | Some(ColorModel::HSLAHex)
        | Some(ColorModel::YCgCoA)
        | Some(ColorModel::YcCbcCrc)
        | Some(ColorModel::ICtCp)
        | Some(ColorModel::CIEXYZ)
        | Some(ColorModel::CIEXYY) => {
            return Err(Ktx2TextureError::Unsupported(format!(
                "{:?}",
                data_format_descriptor.header.color_model
            )));
        }
        Some(ColorModel::XYZW) => {
            // Same number of channels in both texel block dimensions and sample info descriptions
            assert_eq!(
                data_format_descriptor.header.texel_block_dimensions[0].get() as usize,
                sample_information.len()
            );
            match sample_information.len() {
                4 => {
                    // Only RGBA or BGRA channels allowed
                    assert_eq!(sample_information[0].channel_type, 0);
                    assert_eq!(sample_information[1].channel_type, 1);
                    assert_eq!(sample_information[2].channel_type, 2);
                    assert_eq!(sample_information[3].channel_type, 3);
                    // Only same bit length for all channels
                    assert!(
                        sample_information[0].bit_length == sample_information[1].bit_length
                            && sample_information[0].bit_length == sample_information[2].bit_length
                            && sample_information[0].bit_length == sample_information[3].bit_length
                    );
                    // Only same channel type qualifiers for all channels
                    assert!(
                        sample_information[0].channel_type_qualifiers
                            == sample_information[1].channel_type_qualifiers
                            && sample_information[0].channel_type_qualifiers
                                == sample_information[2].channel_type_qualifiers
                            && sample_information[0].channel_type_qualifiers
                                == sample_information[3].channel_type_qualifiers
                    );
                    // Only same sample range for all channels
                    assert!(
                        sample_information[0].lower == sample_information[1].lower
                            && sample_information[0].lower == sample_information[2].lower
                            && sample_information[0].lower == sample_information[3].lower
                    );
                    assert!(
                        sample_information[0].upper == sample_information[1].upper
                            && sample_information[0].upper == sample_information[2].upper
                            && sample_information[0].upper == sample_information[3].upper
                    );

                    let sample = &sample_information[0];
                    let data_type = sample_information_to_data_type(sample, false)?;
                    match sample.bit_length.get() {
                        8 => match data_type {
                            DataType::Unorm => TextureFormat::Rgba8Unorm,
                            DataType::UnormSrgb => {
                                return Err(Ktx2TextureError::Unsupported(
                                    "UnormSrgb not supported for XYZW".to_string(),
                                ));
                            }
                            DataType::Snorm => TextureFormat::Rgba8Snorm,
                            DataType::Float => {
                                return Err(Ktx2TextureError::Unsupported(
                                    "Float not supported for Rgba8/Bgra8".to_string(),
                                ));
                            }
                            DataType::Uint => TextureFormat::Rgba8Uint,
                            DataType::Sint => TextureFormat::Rgba8Sint,
                        },
                        16 => match data_type {
                            DataType::Unorm => TextureFormat::Rgba16Unorm,
                            DataType::UnormSrgb => {
                                return Err(Ktx2TextureError::Unsupported(
                                    "UnormSrgb not supported for Rgba16/Bgra16".to_string(),
                                ));
                            }
                            DataType::Snorm => TextureFormat::Rgba16Snorm,
                            DataType::Float => TextureFormat::Rgba16Float,
                            DataType::Uint => TextureFormat::Rgba16Uint,
                            DataType::Sint => TextureFormat::Rgba16Sint,
                        },
                        32 => match data_type {
                            DataType::Unorm => {
                                return Err(Ktx2TextureError::Unsupported(
                                    "Unorm not supported for Rgba32/Bgra32".to_string(),
                                ));
                            }
                            DataType::UnormSrgb => {
                                return Err(Ktx2TextureError::Unsupported(
                                    "UnormSrgb not supported for Rgba32/Bgra32".to_string(),
                                ));
                            }
                            DataType::Snorm => {
                                return Err(Ktx2TextureError::Unsupported(
                                    "Snorm not supported for Rgba32/Bgra32".to_string(),
                                ));
                            }
                            DataType::Float => TextureFormat::Rgba32Float,
                            DataType::Uint => TextureFormat::Rgba32Uint,
                            DataType::Sint => TextureFormat::Rgba32Sint,
                        },
                        v => {
                            return Err(Ktx2TextureError::Unsupported(format!(
                                "Unsupported sample bit length for XYZW 4-channel format: {v}",
                            )));
                        }
                    }
                }
                v => {
                    return Err(Ktx2TextureError::Unsupported(format!(
                        "Unsupported channel count for XYZW format: {v}",
                    )));
                }
            }
        }
        Some(ColorModel::BC1A) => {
            if is_srgb {
                TextureFormat::Bc1RgbaUnormSrgb
            } else {
                TextureFormat::Bc1RgbaUnorm
            }
        }
        Some(ColorModel::BC2) => {
            if is_srgb {
                TextureFormat::Bc2RgbaUnormSrgb
            } else {
                TextureFormat::Bc2RgbaUnorm
            }
        }
        Some(ColorModel::BC3) => {
            if is_srgb {
                TextureFormat::Bc3RgbaUnormSrgb
            } else {
                TextureFormat::Bc3RgbaUnorm
            }
        }
        Some(ColorModel::BC4) => {
            if sample_information[0].lower == 0 {
                TextureFormat::Bc4RUnorm
            } else {
                TextureFormat::Bc4RSnorm
            }
        }
        // FIXME: Red and green channels can be swapped for ATI2n/3Dc
        Some(ColorModel::BC5) => {
            if sample_information[0].lower == 0 {
                TextureFormat::Bc5RgUnorm
            } else {
                TextureFormat::Bc5RgSnorm
            }
        }
        Some(ColorModel::BC6H) => {
            if sample_information[0].lower == 0 {
                TextureFormat::Bc6hRgbUfloat
            } else {
                TextureFormat::Bc6hRgbFloat
            }
        }
        Some(ColorModel::BC7) => {
            if is_srgb {
                TextureFormat::Bc7RgbaUnormSrgb
            } else {
                TextureFormat::Bc7RgbaUnorm
            }
        }
        // ETC1 a subset of ETC2 only supporting Rgb8
        Some(ColorModel::ETC1) => {
            if is_srgb {
                TextureFormat::Etc2Rgb8UnormSrgb
            } else {
                TextureFormat::Etc2Rgb8Unorm
            }
        }
        Some(ColorModel::ETC2) => match sample_information.len() {
            1 => {
                let sample = &sample_information[0];
                match sample.channel_type {
                    0 => {
                        if sample_information[0]
                            .channel_type_qualifiers
                            .contains(ChannelTypeQualifiers::SIGNED)
                        {
                            TextureFormat::EacR11Snorm
                        } else {
                            TextureFormat::EacR11Unorm
                        }
                    }
                    2 => {
                        if is_srgb {
                            TextureFormat::Etc2Rgb8UnormSrgb
                        } else {
                            TextureFormat::Etc2Rgb8Unorm
                        }
                    }
                    _ => {
                        return Err(Ktx2TextureError::Invalid(format!(
                            "Invalid ETC2 sample channel type: {}",
                            sample.channel_type
                        )))
                    }
                }
            }
            2 => {
                let sample0 = &sample_information[0];
                let sample1 = &sample_information[1];
                if sample0.channel_type == 0 && sample1.channel_type == 1 {
                    if sample0
                        .channel_type_qualifiers
                        .contains(ChannelTypeQualifiers::SIGNED)
                    {
                        TextureFormat::EacRg11Snorm
                    } else {
                        TextureFormat::EacRg11Unorm
                    }
                } else if sample0.channel_type == 2 && sample1.channel_type == 15 {
                    if is_srgb {
                        TextureFormat::Etc2Rgb8A1UnormSrgb
                    } else {
                        TextureFormat::Etc2Rgb8A1Unorm
                    }
                } else if sample0.channel_type == 15 && sample1.channel_type == 2 {
                    if is_srgb {
                        TextureFormat::Etc2Rgba8UnormSrgb
                    } else {
                        TextureFormat::Etc2Rgba8Unorm
                    }
                } else {
                    return Err(Ktx2TextureError::Invalid(format!(
                        "Invalid ETC2 2-sample channel types: {} {}",
                        sample0.channel_type, sample1.channel_type
                    )));
                }
            }
            v => {
                return Err(Ktx2TextureError::Invalid(format!(
                    "Invalid channel count for ETC2 format: {v}",
                )));
            }
        },
        Some(ColorModel::ASTC) => TextureFormat::Astc {
            block: match (
                data_format_descriptor.header.texel_block_dimensions[0].get(),
                data_format_descriptor.header.texel_block_dimensions[1].get(),
            ) {
                (4, 4) => AstcBlock::B4x4,
                (5, 4) => AstcBlock::B5x4,
                (5, 5) => AstcBlock::B5x5,
                (6, 5) => AstcBlock::B6x5,
                (8, 5) => AstcBlock::B8x5,
                (8, 8) => AstcBlock::B8x8,
                (10, 5) => AstcBlock::B10x5,
                (10, 6) => AstcBlock::B10x6,
                (10, 8) => AstcBlock::B10x8,
                (10, 10) => AstcBlock::B10x10,
                (12, 10) => AstcBlock::B12x10,
                (12, 12) => AstcBlock::B12x12,
                d => {
                    return Err(Ktx2TextureError::Invalid(format!(
                        "Invalid ASTC dimension: {}x{}",
                        d.0, d.1
                    )))
                }
            },
            channel: if is_srgb {
                AstcChannel::UnormSrgb
            } else {
                AstcChannel::Unorm
            },
        },
        Some(ColorModel::ETC1S) => {
            // ETC1S is a subset of ETC1 which is a subset of ETC2
            if is_srgb {
                TextureFormat::Etc2Rgb8UnormSrgb
            } else {
                TextureFormat::Etc2Rgb8Unorm
            }
        }
        Some(ColorModel::PVRTC) => {
            return Err(Ktx2TextureError::Unsupported(
                "PVRTC is not supported".to_string(),
            ));
        }
        Some(ColorModel::PVRTC2) => {
            return Err(Ktx2TextureError::Unsupported(
                "PVRTC2 is not supported".to_string(),
            ));
        }
        Some(ColorModel::UASTC) => {
            // UASTC, as far as KTX2 spec is concerned, is always 4x4 LDR. Tools that output formats like
            // UASTC 4x4 HDR, UASTC 8x8 LDR set the ColorModel to a value in the "Proprietary" space.
            let block_width = data_format_descriptor.header.texel_block_dimensions[0].get();
            let block_height = data_format_descriptor.header.texel_block_dimensions[1].get();
            let data_format = match sample_information[0].channel_type {
                0 => DataFormat::Rgb,
                3 => DataFormat::Rgba,
                4 => DataFormat::Rrr,
                5 => DataFormat::Rrrg,
                6 => DataFormat::Rg,
                channel_type => {
                    return Err(Ktx2TextureError::Invalid(format!(
                        "Invalid KTX2 UASTC channel type: {channel_type}",
                    )))
                }
            };
            match (block_width, block_height) {
                (4, 4) => {
                    // UASTC always needs transcoding to ASTC
                    return Err(Ktx2TextureError::RequiresTranscoding(
                        Ktx2TranscodingHint::UastcLdr4x4 {
                            data_format,
                            is_srgb,
                        },
                    ));
                }
                (_, _) => {
                    return Err(Ktx2TextureError::Invalid(format!(
                        "Invalid UASTC block size: {block_width}x{block_height}",
                    )))
                }
            }
        }
        None => {
            return Err(Ktx2TextureError::Invalid(
                "Unspecified KTX2 color model".to_string(),
            ));
        }
        _ => {
            return Err(Ktx2TextureError::Unsupported(format!(
                "Unknown KTX2 color model: {:?}",
                data_format_descriptor.header.color_model
            )));
        }
    })
}

/// Translates a [`ktx2::Format`] to its corresponding [`wgpu_types::TextureFormat`].
pub fn ktx2_format_to_texture_format(
    ktx2_format: ktx2::Format,
) -> Result<TextureFormat, Ktx2TextureError> {
    fn no_wgpu_format(fmt: &str) -> Result<TextureFormat, Ktx2TextureError> {
        Err(Ktx2TextureError::Unsupported(format!(
            "ktx2::Format::{} has no matching wgpu type",
            fmt
        )))
    }
    Ok(match ktx2_format {
        ktx2::Format::R4G4_UNORM_PACK8 => no_wgpu_format("R4G4_UNORM_PACK8")?,
        ktx2::Format::R4G4B4A4_UNORM_PACK16 => no_wgpu_format("R4G4B4A4_UNORM_PACK16")?,
        ktx2::Format::B4G4R4A4_UNORM_PACK16 => no_wgpu_format("B4G4R4A4_UNORM_PACK16")?,
        ktx2::Format::R5G6B5_UNORM_PACK16 => no_wgpu_format("R5G6B5_UNORM_PACK16")?,
        ktx2::Format::B5G6R5_UNORM_PACK16 => no_wgpu_format("B5G6R5_UNORM_PACK16")?,
        ktx2::Format::R5G5B5A1_UNORM_PACK16 => no_wgpu_format("R5G5B5A1_UNORM_PACK16")?,
        ktx2::Format::B5G5R5A1_UNORM_PACK16 => no_wgpu_format("B5G5R5A1_UNORM_PACK16")?,
        ktx2::Format::A1R5G5B5_UNORM_PACK16 => no_wgpu_format("A1R5G5B5_UNORM_PACK16")?,

        ktx2::Format::R8_UNORM => TextureFormat::R8Unorm,
        ktx2::Format::R8_SRGB => {
            return Err(Ktx2TextureError::RequiresTranscoding(
                Ktx2TranscodingHint::R8UnormSrgb,
            ));
        }
        ktx2::Format::R8_SNORM => TextureFormat::R8Snorm,
        ktx2::Format::R8_UINT => TextureFormat::R8Uint,
        ktx2::Format::R8_SINT => TextureFormat::R8Sint,
        ktx2::Format::R8G8_UNORM => TextureFormat::Rg8Unorm,
        ktx2::Format::R8G8_SRGB => {
            return Err(Ktx2TextureError::RequiresTranscoding(
                Ktx2TranscodingHint::Rg8UnormSrgb,
            ));
        }
        ktx2::Format::R8G8_SNORM => TextureFormat::Rg8Snorm,
        ktx2::Format::R8G8_UINT => TextureFormat::Rg8Uint,
        ktx2::Format::R8G8_SINT => TextureFormat::Rg8Sint,
        ktx2::Format::R8G8B8_UNORM => {
            return Err(Ktx2TextureError::RequiresTranscoding(
                Ktx2TranscodingHint::Rgb8 { is_srgb: false },
            ));
        }
        ktx2::Format::R8G8B8_SRGB => {
            return Err(Ktx2TextureError::RequiresTranscoding(
                Ktx2TranscodingHint::Rgb8 { is_srgb: true },
            ));
        }
        ktx2::Format::R8G8B8A8_UNORM => TextureFormat::Rgba8Unorm,
        ktx2::Format::R8G8B8A8_SRGB => TextureFormat::Rgba8UnormSrgb,
        ktx2::Format::R8G8B8A8_SNORM => TextureFormat::Rgba8Snorm,
        ktx2::Format::R8G8B8A8_UINT => TextureFormat::Rgba8Uint,
        ktx2::Format::R8G8B8A8_SINT => TextureFormat::Rgba8Sint,
        ktx2::Format::B8G8R8A8_UNORM => TextureFormat::Bgra8Unorm,
        ktx2::Format::B8G8R8A8_SRGB => TextureFormat::Bgra8UnormSrgb,
        ktx2::Format::A2R10G10B10_UINT_PACK32 => TextureFormat::Rgb10a2Uint,
        ktx2::Format::A2R10G10B10_UNORM_PACK32 => TextureFormat::Rgb10a2Unorm,
        ktx2::Format::A2B10G10R10_UNORM_PACK32 => no_wgpu_format("A2B10G10R10_UNORM_PACK32")?,
        ktx2::Format::A2R10G10B10_SNORM_PACK32 => no_wgpu_format("A2R10G10B10_SNORM_PACK32")?,
        ktx2::Format::A2B10G10R10_SNORM_PACK32 => no_wgpu_format("A2B10G10R10_SNORM_PACK32")?,
        ktx2::Format::A2R10G10B10_SINT_PACK32 => no_wgpu_format("A2R10G10B10_SINT_PACK32")?,
        ktx2::Format::A2B10G10R10_UINT_PACK32 => no_wgpu_format("A2B10G10R10_UINT_PACK32")?,
        ktx2::Format::A2B10G10R10_SINT_PACK32 => no_wgpu_format("A2B10G10R10_SINT_PACK32")?,

        ktx2::Format::R16_UNORM => TextureFormat::R16Unorm,
        ktx2::Format::R16_SNORM => TextureFormat::R16Snorm,
        ktx2::Format::R16_UINT => TextureFormat::R16Uint,
        ktx2::Format::R16_SINT => TextureFormat::R16Sint,
        ktx2::Format::R16_SFLOAT => TextureFormat::R16Float,
        ktx2::Format::R16G16_UNORM => TextureFormat::Rg16Unorm,
        ktx2::Format::R16G16_SNORM => TextureFormat::Rg16Snorm,
        ktx2::Format::R16G16_UINT => TextureFormat::Rg16Uint,
        ktx2::Format::R16G16_SINT => TextureFormat::Rg16Sint,
        ktx2::Format::R16G16_SFLOAT => TextureFormat::Rg16Float,

        ktx2::Format::R16G16B16A16_UNORM => TextureFormat::Rgba16Unorm,
        ktx2::Format::R16G16B16A16_SNORM => TextureFormat::Rgba16Snorm,
        ktx2::Format::R16G16B16A16_UINT => TextureFormat::Rgba16Uint,
        ktx2::Format::R16G16B16A16_SINT => TextureFormat::Rgba16Sint,
        ktx2::Format::R16G16B16A16_SFLOAT => TextureFormat::Rgba16Float,
        ktx2::Format::R32_UINT => TextureFormat::R32Uint,
        ktx2::Format::R32_SINT => TextureFormat::R32Sint,
        ktx2::Format::R32_SFLOAT => TextureFormat::R32Float,
        ktx2::Format::R32G32_UINT => TextureFormat::Rg32Uint,
        ktx2::Format::R32G32_SINT => TextureFormat::Rg32Sint,
        ktx2::Format::R32G32_SFLOAT => TextureFormat::Rg32Float,

        ktx2::Format::R32G32B32_UINT => {
            return Err(Ktx2TextureError::RequiresTranscoding(
                Ktx2TranscodingHint::Rgb32Uint,
            ));
        }
        ktx2::Format::R32G32B32_SINT => {
            return Err(Ktx2TextureError::RequiresTranscoding(
                Ktx2TranscodingHint::Rgb32Sint,
            ));
        }
        ktx2::Format::R32G32B32_SFLOAT => {
            return Err(Ktx2TextureError::RequiresTranscoding(
                Ktx2TranscodingHint::Rgb32Float,
            ));
        }

        ktx2::Format::R32G32B32A32_UINT => TextureFormat::Rgba32Uint,
        ktx2::Format::R32G32B32A32_SINT => TextureFormat::Rgba32Sint,
        ktx2::Format::R32G32B32A32_SFLOAT => TextureFormat::Rgba32Float,

        ktx2::Format::R64_UINT => TextureFormat::R64Uint,
        ktx2::Format::R64_SINT => no_wgpu_format("R64_SINT")?,
        ktx2::Format::R64_SFLOAT => no_wgpu_format("R64_SFLOAT")?,
        ktx2::Format::R64G64_UINT => no_wgpu_format("R64G64_UINT")?,
        ktx2::Format::R64G64_SINT => no_wgpu_format("R64G64_SINT")?,
        ktx2::Format::R64G64_SFLOAT => no_wgpu_format("R64G64_SFLOAT")?,
        ktx2::Format::R64G64B64_UINT => no_wgpu_format("R64G64B64_UINT")?,
        ktx2::Format::R64G64B64_SINT => no_wgpu_format("R64G64B64_SINT")?,
        ktx2::Format::R64G64B64_SFLOAT => no_wgpu_format("R64G64B64_SFLOAT")?,
        ktx2::Format::R64G64B64A64_UINT => no_wgpu_format("R64G64B64A64_UINT")?,
        ktx2::Format::R64G64B64A64_SINT => no_wgpu_format("R64G64B64A64_SINT")?,
        ktx2::Format::R64G64B64A64_SFLOAT => no_wgpu_format("R64G64B64A64_SFLOAT")?,

        ktx2::Format::B10G11R11_UFLOAT_PACK32 => TextureFormat::Rg11b10Ufloat,
        ktx2::Format::E5B9G9R9_UFLOAT_PACK32 => TextureFormat::Rgb9e5Ufloat,

        ktx2::Format::S8_UINT => TextureFormat::Stencil8,

        ktx2::Format::X8_D24_UNORM_PACK32 => TextureFormat::Depth24Plus,
        ktx2::Format::D16_UNORM => TextureFormat::Depth16Unorm,
        ktx2::Format::D16_UNORM_S8_UINT => no_wgpu_format("D16_UNORM_S8_UINT")?,
        ktx2::Format::D32_SFLOAT => TextureFormat::Depth32Float,
        ktx2::Format::D24_UNORM_S8_UINT => TextureFormat::Depth24PlusStencil8,
        ktx2::Format::D32_SFLOAT_S8_UINT => TextureFormat::Depth32FloatStencil8,

        ktx2::Format::BC1_RGB_UNORM_BLOCK => no_wgpu_format("BC1_RGB_UNORM_BLOCK")?,
        ktx2::Format::BC1_RGB_SRGB_BLOCK => no_wgpu_format("BC1_RGB_SRGB_BLOCK")?,
        ktx2::Format::BC1_RGBA_UNORM_BLOCK => TextureFormat::Bc1RgbaUnorm,
        ktx2::Format::BC1_RGBA_SRGB_BLOCK => TextureFormat::Bc1RgbaUnormSrgb,
        ktx2::Format::BC2_UNORM_BLOCK => TextureFormat::Bc2RgbaUnorm,
        ktx2::Format::BC2_SRGB_BLOCK => TextureFormat::Bc2RgbaUnormSrgb,
        ktx2::Format::BC3_UNORM_BLOCK => TextureFormat::Bc3RgbaUnorm,
        ktx2::Format::BC3_SRGB_BLOCK => TextureFormat::Bc3RgbaUnormSrgb,
        ktx2::Format::BC4_UNORM_BLOCK => TextureFormat::Bc4RUnorm,
        ktx2::Format::BC4_SNORM_BLOCK => TextureFormat::Bc4RSnorm,
        ktx2::Format::BC5_UNORM_BLOCK => TextureFormat::Bc5RgUnorm,
        ktx2::Format::BC5_SNORM_BLOCK => TextureFormat::Bc5RgSnorm,
        ktx2::Format::BC6H_UFLOAT_BLOCK => TextureFormat::Bc6hRgbUfloat,
        ktx2::Format::BC6H_SFLOAT_BLOCK => TextureFormat::Bc6hRgbFloat,
        ktx2::Format::BC7_UNORM_BLOCK => TextureFormat::Bc7RgbaUnorm,
        ktx2::Format::BC7_SRGB_BLOCK => TextureFormat::Bc7RgbaUnormSrgb,
        ktx2::Format::ETC2_R8G8B8_UNORM_BLOCK => TextureFormat::Etc2Rgb8Unorm,
        ktx2::Format::ETC2_R8G8B8_SRGB_BLOCK => TextureFormat::Etc2Rgb8UnormSrgb,
        ktx2::Format::ETC2_R8G8B8A1_UNORM_BLOCK => TextureFormat::Etc2Rgb8A1Unorm,
        ktx2::Format::ETC2_R8G8B8A1_SRGB_BLOCK => TextureFormat::Etc2Rgb8A1UnormSrgb,
        ktx2::Format::ETC2_R8G8B8A8_UNORM_BLOCK => TextureFormat::Etc2Rgba8Unorm,
        ktx2::Format::ETC2_R8G8B8A8_SRGB_BLOCK => TextureFormat::Etc2Rgba8UnormSrgb,
        ktx2::Format::EAC_R11_UNORM_BLOCK => TextureFormat::EacR11Unorm,
        ktx2::Format::EAC_R11_SNORM_BLOCK => TextureFormat::EacR11Snorm,
        ktx2::Format::EAC_R11G11_UNORM_BLOCK => TextureFormat::EacRg11Unorm,
        ktx2::Format::EAC_R11G11_SNORM_BLOCK => TextureFormat::EacRg11Snorm,

        // ASTC 4x4
        ktx2::Format::ASTC_4x4_UNORM_BLOCK => TextureFormat::Astc {
            block: AstcBlock::B4x4,
            channel: AstcChannel::Unorm,
        },
        ktx2::Format::ASTC_4x4_SRGB_BLOCK => TextureFormat::Astc {
            block: AstcBlock::B4x4,
            channel: AstcChannel::UnormSrgb,
        },
        ktx2::Format::ASTC_4x4_SFLOAT_BLOCK => TextureFormat::Astc {
            block: AstcBlock::B4x4,
            channel: AstcChannel::Hdr,
        },
        // ASTC 5x4
        ktx2::Format::ASTC_5x4_UNORM_BLOCK => TextureFormat::Astc {
            block: AstcBlock::B5x4,
            channel: AstcChannel::Unorm,
        },
        ktx2::Format::ASTC_5x4_SRGB_BLOCK => TextureFormat::Astc {
            block: AstcBlock::B5x4,
            channel: AstcChannel::UnormSrgb,
        },
        ktx2::Format::ASTC_5x4_SFLOAT_BLOCK => TextureFormat::Astc {
            block: AstcBlock::B5x4,
            channel: AstcChannel::Hdr,
        },
        // ASTC 5x5
        ktx2::Format::ASTC_5x5_UNORM_BLOCK => TextureFormat::Astc {
            block: AstcBlock::B5x5,
            channel: AstcChannel::Unorm,
        },
        ktx2::Format::ASTC_5x5_SRGB_BLOCK => TextureFormat::Astc {
            block: AstcBlock::B5x5,
            channel: AstcChannel::UnormSrgb,
        },
        ktx2::Format::ASTC_5x5_SFLOAT_BLOCK => TextureFormat::Astc {
            block: AstcBlock::B5x5,
            channel: AstcChannel::Hdr,
        },
        // ASTC 6x5
        ktx2::Format::ASTC_6x5_UNORM_BLOCK => TextureFormat::Astc {
            block: AstcBlock::B6x5,
            channel: AstcChannel::Unorm,
        },
        ktx2::Format::ASTC_6x5_SRGB_BLOCK => TextureFormat::Astc {
            block: AstcBlock::B6x5,
            channel: AstcChannel::UnormSrgb,
        },
        ktx2::Format::ASTC_6x5_SFLOAT_BLOCK => TextureFormat::Astc {
            block: AstcBlock::B6x5,
            channel: AstcChannel::Hdr,
        },
        // ASTC 6x6
        ktx2::Format::ASTC_6x6_UNORM_BLOCK => TextureFormat::Astc {
            block: AstcBlock::B6x6,
            channel: AstcChannel::Unorm,
        },
        ktx2::Format::ASTC_6x6_SRGB_BLOCK => TextureFormat::Astc {
            block: AstcBlock::B6x6,
            channel: AstcChannel::UnormSrgb,
        },
        ktx2::Format::ASTC_6x6_SFLOAT_BLOCK => TextureFormat::Astc {
            block: AstcBlock::B6x6,
            channel: AstcChannel::Hdr,
        },
        // ASTC 8x5
        ktx2::Format::ASTC_8x5_UNORM_BLOCK => TextureFormat::Astc {
            block: AstcBlock::B8x5,
            channel: AstcChannel::Unorm,
        },
        ktx2::Format::ASTC_8x5_SRGB_BLOCK => TextureFormat::Astc {
            block: AstcBlock::B8x5,
            channel: AstcChannel::UnormSrgb,
        },
        ktx2::Format::ASTC_8x5_SFLOAT_BLOCK => TextureFormat::Astc {
            block: AstcBlock::B8x5,
            channel: AstcChannel::Hdr,
        },
        // ASTC 8x6
        ktx2::Format::ASTC_8x6_UNORM_BLOCK => TextureFormat::Astc {
            block: AstcBlock::B8x6,
            channel: AstcChannel::Unorm,
        },
        ktx2::Format::ASTC_8x6_SRGB_BLOCK => TextureFormat::Astc {
            block: AstcBlock::B8x6,
            channel: AstcChannel::UnormSrgb,
        },
        ktx2::Format::ASTC_8x6_SFLOAT_BLOCK => TextureFormat::Astc {
            block: AstcBlock::B8x6,
            channel: AstcChannel::Hdr,
        },
        // ASTC 8x8
        ktx2::Format::ASTC_8x8_UNORM_BLOCK => TextureFormat::Astc {
            block: AstcBlock::B8x8,
            channel: AstcChannel::Unorm,
        },
        ktx2::Format::ASTC_8x8_SRGB_BLOCK => TextureFormat::Astc {
            block: AstcBlock::B8x8,
            channel: AstcChannel::UnormSrgb,
        },
        ktx2::Format::ASTC_8x8_SFLOAT_BLOCK => TextureFormat::Astc {
            block: AstcBlock::B8x8,
            channel: AstcChannel::Hdr,
        },
        // ASTC 10x5
        ktx2::Format::ASTC_10x5_UNORM_BLOCK => TextureFormat::Astc {
            block: AstcBlock::B10x5,
            channel: AstcChannel::Unorm,
        },
        ktx2::Format::ASTC_10x5_SRGB_BLOCK => TextureFormat::Astc {
            block: AstcBlock::B10x5,
            channel: AstcChannel::UnormSrgb,
        },
        ktx2::Format::ASTC_10x5_SFLOAT_BLOCK => TextureFormat::Astc {
            block: AstcBlock::B10x5,
            channel: AstcChannel::Hdr,
        },
        // ASTC 10x6
        ktx2::Format::ASTC_10x6_UNORM_BLOCK => TextureFormat::Astc {
            block: AstcBlock::B10x6,
            channel: AstcChannel::Unorm,
        },
        ktx2::Format::ASTC_10x6_SRGB_BLOCK => TextureFormat::Astc {
            block: AstcBlock::B10x6,
            channel: AstcChannel::UnormSrgb,
        },
        ktx2::Format::ASTC_10x6_SFLOAT_BLOCK => TextureFormat::Astc {
            block: AstcBlock::B10x6,
            channel: AstcChannel::Hdr,
        },
        // ASTC 10x8
        ktx2::Format::ASTC_10x8_UNORM_BLOCK => TextureFormat::Astc {
            block: AstcBlock::B10x8,
            channel: AstcChannel::Unorm,
        },
        ktx2::Format::ASTC_10x8_SRGB_BLOCK => TextureFormat::Astc {
            block: AstcBlock::B10x8,
            channel: AstcChannel::UnormSrgb,
        },
        ktx2::Format::ASTC_10x8_SFLOAT_BLOCK => TextureFormat::Astc {
            block: AstcBlock::B10x8,
            channel: AstcChannel::Hdr,
        },
        // ASTC 10x10
        ktx2::Format::ASTC_10x10_UNORM_BLOCK => TextureFormat::Astc {
            block: AstcBlock::B10x10,
            channel: AstcChannel::Unorm,
        },
        ktx2::Format::ASTC_10x10_SRGB_BLOCK => TextureFormat::Astc {
            block: AstcBlock::B10x10,
            channel: AstcChannel::UnormSrgb,
        },
        ktx2::Format::ASTC_10x10_SFLOAT_BLOCK => TextureFormat::Astc {
            block: AstcBlock::B10x10,
            channel: AstcChannel::Hdr,
        },
        // ASTC 12x10
        ktx2::Format::ASTC_12x10_UNORM_BLOCK => TextureFormat::Astc {
            block: AstcBlock::B12x10,
            channel: AstcChannel::Unorm,
        },
        ktx2::Format::ASTC_12x10_SRGB_BLOCK => TextureFormat::Astc {
            block: AstcBlock::B12x10,
            channel: AstcChannel::UnormSrgb,
        },
        ktx2::Format::ASTC_12x10_SFLOAT_BLOCK => TextureFormat::Astc {
            block: AstcBlock::B12x10,
            channel: AstcChannel::Hdr,
        },
        // ASTC 12x12
        ktx2::Format::ASTC_12x12_UNORM_BLOCK => TextureFormat::Astc {
            block: AstcBlock::B12x12,
            channel: AstcChannel::Unorm,
        },
        ktx2::Format::ASTC_12x12_SRGB_BLOCK => TextureFormat::Astc {
            block: AstcBlock::B12x12,
            channel: AstcChannel::UnormSrgb,
        },
        ktx2::Format::ASTC_12x12_SFLOAT_BLOCK => TextureFormat::Astc {
            block: AstcBlock::B12x12,
            channel: AstcChannel::Hdr,
        },

        // This catch-all is needed because ktx2::Format isn't an actual enum. There is
        // a possibility for a format id that does not map to a defined format.
        other_format => {
            return Err(Ktx2TextureError::Unsupported(format!(
                "KTX2 format {:?} has no matching wgpu type",
                other_format
            )));
        }
    })
}

#[cfg(test)]
mod tests {
    use wgpu_types::TextureFormat;

    use crate::CompressedImageFormats;

    use super::ktx2_buffer_to_image;

    #[test]
    fn test_ktx_levels() {
        // R8UnormSrgb texture with 4x4 pixels data and 3 levels of mipmaps
        let buffer = vec![
            0xab, 0x4b, 0x54, 0x58, 0x20, 0x32, 0x30, 0xbb, 0x0d, 10, 0x1a, 10, 0x0f, 0, 0, 0, 1,
            0, 0, 0, 4, 0, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 3, 0, 0, 0, 0, 0,
            0, 0, 0x98, 0, 0, 0, 0x2c, 0, 0, 0, 0xc4, 0, 0, 0, 0x5c, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0x28, 1, 0, 0, 0, 0, 0, 0, 0x10, 0, 0, 0, 0, 0, 0, 0, 0x10,
            0, 0, 0, 0, 0, 0, 0, 0x24, 1, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0, 0,
            0, 0, 0, 0x20, 1, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0,
            0x2c, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0x28, 0, 1, 1, 2, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 7, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0xff, 0, 0, 0, 0x12, 0, 0, 0, 0x4b, 0x54, 0x58,
            0x6f, 0x72, 0x69, 0x65, 0x6e, 0x74, 0x61, 0x74, 0x69, 0x6f, 0x6e, 0, 0x72, 0x64, 0, 0,
            0, 0x10, 0, 0, 0, 0x4b, 0x54, 0x58, 0x73, 0x77, 0x69, 0x7a, 0x7a, 0x6c, 0x65, 0, 0x72,
            0x72, 0x72, 0x31, 0, 0x2c, 0, 0, 0, 0x4b, 0x54, 0x58, 0x77, 0x72, 0x69, 0x74, 0x65,
            0x72, 0, 0x74, 0x6f, 0x6b, 0x74, 0x78, 0x20, 0x76, 0x34, 0x2e, 0x33, 0x2e, 0x30, 0x7e,
            0x32, 0x38, 0x20, 0x2f, 0x20, 0x6c, 0x69, 0x62, 0x6b, 0x74, 0x78, 0x20, 0x76, 0x34,
            0x2e, 0x33, 0x2e, 0x30, 0x7e, 0x31, 0, 0x4a, 0, 0, 0, 0x4a, 0x4a, 0x4a, 0x4a, 0x4a,
            0x4a, 0x4a, 0x4a, 0x4a, 0x4a, 0x4a, 0x4a, 0x4a, 0x4a, 0x4a, 0x4a, 0x4a, 0x4a, 0x4a,
            0x4a,
        ];
        let supported_compressed_formats = CompressedImageFormats::empty();
        let result = ktx2_buffer_to_image(&buffer, supported_compressed_formats);
        let image = result.unwrap();

        assert_eq!(image.texture_descriptor.format, TextureFormat::R8Unorm);
        assert_eq!(image.texture_descriptor.mip_level_count, 3);
    }
}
