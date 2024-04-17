#[cfg(any(feature = "flate2", feature = "ruzstd"))]
use std::io::Read;

#[cfg(feature = "basis-universal")]
use basis_universal::{
    DecodeFlags, LowLevelUastcTranscoder, SliceParametersUastc, TranscoderBlockFormat,
};
use bevy_color::Srgba;
use bevy_utils::default;
#[cfg(any(feature = "flate2", feature = "ruzstd"))]
use ktx2::SupercompressionScheme;
use ktx2::{
    BasicDataFormatDescriptor, ChannelTypeQualifiers, ColorModel, DataFormatDescriptorHeader,
    Header, SampleInformation,
};
use wgpu::{
    AstcBlock, AstcChannel, Extent3d, TextureDimension, TextureFormat, TextureViewDescriptor,
    TextureViewDimension,
};

use super::{CompressedImageFormats, DataFormat, Image, TextureError, TranscodeFormat};

pub fn ktx2_buffer_to_image(
    buffer: &[u8],
    supported_compressed_formats: CompressedImageFormats,
    is_srgb: bool,
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
    let depth = depth.max(1);

    // Handle supercompression
    let mut levels = Vec::new();
    if let Some(supercompression_scheme) = supercompression_scheme {
        for (_level, _level_data) in ktx2.levels().enumerate() {
            match supercompression_scheme {
                #[cfg(feature = "flate2")]
                SupercompressionScheme::ZLIB => {
                    let mut decoder = flate2::bufread::ZlibDecoder::new(_level_data);
                    let mut decompressed = Vec::new();
                    decoder.read_to_end(&mut decompressed).map_err(|err| {
                        TextureError::SuperDecompressionError(format!(
                            "Failed to decompress {supercompression_scheme:?} for mip {_level}: {err:?}",
                        ))
                    })?;
                    levels.push(decompressed);
                }
                #[cfg(feature = "ruzstd")]
                SupercompressionScheme::Zstandard => {
                    let mut cursor = std::io::Cursor::new(_level_data);
                    let mut decoder = ruzstd::StreamingDecoder::new(&mut cursor)
                        .map_err(|err| TextureError::SuperDecompressionError(err.to_string()))?;
                    let mut decompressed = Vec::new();
                    decoder.read_to_end(&mut decompressed).map_err(|err| {
                        TextureError::SuperDecompressionError(format!(
                            "Failed to decompress {supercompression_scheme:?} for mip {_level}: {err:?}",
                        ))
                    })?;
                    levels.push(decompressed);
                }
                _ => {
                    return Err(TextureError::SuperDecompressionError(format!(
                        "Unsupported supercompression scheme: {supercompression_scheme:?}",
                    )));
                }
            }
        }
    } else {
        levels = ktx2.levels().map(|level| level.to_vec()).collect();
    }

    // Identify the format
    let texture_format = ktx2_get_texture_format(&ktx2, is_srgb).or_else(|error| match error {
        // Transcode if needed and supported
        TextureError::FormatRequiresTranscodingError(transcode_format) => {
            let mut transcoded = vec![Vec::default(); levels.len()];
            let texture_format = match transcode_format {
                TranscodeFormat::R8UnormSrgb => {
                    let (mut original_width, mut original_height) = (width, height);

                    for (level, level_data) in levels.iter().enumerate() {
                        transcoded[level] = level_data
                            .iter()
                            .copied()
                            .map(|v| (Srgba::gamma_function(v as f32 / 255.) * 255.).floor() as u8)
                            .collect::<Vec<u8>>();

                        // Next mip dimensions are half the current, minimum 1x1
                        original_width = (original_width / 2).max(1);
                        original_height = (original_height / 2).max(1);
                    }

                    TextureFormat::R8Unorm
                }
                TranscodeFormat::Rg8UnormSrgb => {
                    let (mut original_width, mut original_height) = (width, height);

                    for (level, level_data) in levels.iter().enumerate() {
                        transcoded[level] = level_data
                            .iter()
                            .copied()
                            .map(|v| (Srgba::gamma_function(v as f32 / 255.) * 255.).floor() as u8)
                            .collect::<Vec<u8>>();

                        // Next mip dimensions are half the current, minimum 1x1
                        original_width = (original_width / 2).max(1);
                        original_height = (original_height / 2).max(1);
                    }

                    TextureFormat::Rg8Unorm
                }
                TranscodeFormat::Rgb8 => {
                    let mut rgba = vec![255u8; width as usize * height as usize * 4];
                    for (level, level_data) in levels.iter().enumerate() {
                        let n_pixels = (width as usize >> level).max(1) * (height as usize >> level).max(1);

                        let mut offset = 0;
                        for _layer in 0..layer_count {
                            for _face in 0..face_count {
                                for i in 0..n_pixels {
                                    rgba[i * 4] = level_data[offset];
                                    rgba[i * 4 + 1] = level_data[offset + 1];
                                    rgba[i * 4 + 2] = level_data[offset + 2];
                                    offset += 3;
                                }
                                transcoded[level].extend_from_slice(&rgba[0..n_pixels * 4]);
                            }
                        }
                    }

                    if is_srgb {
                        TextureFormat::Rgba8UnormSrgb
                    } else {
                        TextureFormat::Rgba8Unorm
                    }
                }
                #[cfg(feature = "basis-universal")]
                TranscodeFormat::Uastc(data_format) => {
                    let (transcode_block_format, texture_format) =
                        get_transcoded_formats(supported_compressed_formats, data_format, is_srgb);
                    let texture_format_info = texture_format;
                    let (block_width_pixels, block_height_pixels) = (
                        texture_format_info.block_dimensions().0,
                        texture_format_info.block_dimensions().1,
                    );
                    // Texture is not a depth or stencil format, it is possible to pass `None` and unwrap
                    let block_bytes = texture_format_info.block_copy_size(None).unwrap();

                    let transcoder = LowLevelUastcTranscoder::new();
                    for (level, level_data) in levels.iter().enumerate() {
                        let (level_width, level_height) = (
                            (width >> level as u32).max(1),
                            (height >> level as u32).max(1),
                        );
                        let (num_blocks_x, num_blocks_y) = (
                            ((level_width + block_width_pixels - 1) / block_width_pixels) .max(1),
                            ((level_height + block_height_pixels - 1) / block_height_pixels) .max(1),
                        );
                        let level_bytes = (num_blocks_x * num_blocks_y * block_bytes) as usize;

                        let mut offset = 0;
                        for _layer in 0..layer_count {
                            for _face in 0..face_count {
                                // NOTE: SliceParametersUastc does not implement Clone nor Copy so
                                // it has to be created per use
                                let slice_parameters = SliceParametersUastc {
                                    num_blocks_x,
                                    num_blocks_y,
                                    has_alpha: false,
                                    original_width: level_width,
                                    original_height: level_height,
                                };
                                transcoder
                                    .transcode_slice(
                                        &level_data[offset..(offset + level_bytes)],
                                        slice_parameters,
                                        DecodeFlags::HIGH_QUALITY,
                                        transcode_block_format,
                                    )
                                    .map(|mut transcoded_level| transcoded[level].append(&mut transcoded_level))
                                    .map_err(|error| {
                                        TextureError::SuperDecompressionError(format!(
                                            "Failed to transcode mip level {level} from UASTC to {transcode_block_format:?}: {error:?}",
                                        ))
                                    })?;
                                offset += level_bytes;
                            }
                        }
                    }
                    texture_format
                }
                // ETC1S is a subset of ETC1 which is a subset of ETC2
                // TODO: Implement transcoding
                TranscodeFormat::Etc1s => {
                    let texture_format = if is_srgb {
                        TextureFormat::Etc2Rgb8UnormSrgb
                    } else {
                        TextureFormat::Etc2Rgb8Unorm
                    };
                    if !supported_compressed_formats.supports(texture_format) {
                        return Err(error);
                    }
                    transcoded = levels.to_vec();
                    texture_format
                }
                #[cfg(not(feature = "basis-universal"))]
                _ => return Err(error),
            };
            levels = transcoded;
            Ok(texture_format)
        }
        _ => Err(error),
    })?;
    if !supported_compressed_formats.supports(texture_format) {
        return Err(TextureError::UnsupportedTextureFormat(format!(
            "Format not supported by this GPU: {texture_format:?}",
        )));
    }

    // Reorder data from KTX2 MipXLayerYFaceZ to wgpu LayerYFaceZMipX
    let texture_format_info = texture_format;
    let (block_width_pixels, block_height_pixels) = (
        texture_format_info.block_dimensions().0 as usize,
        texture_format_info.block_dimensions().1 as usize,
    );
    // Texture is not a depth or stencil format, it is possible to pass `None` and unwrap
    let block_bytes = texture_format_info.block_copy_size(None).unwrap() as usize;

    let mut wgpu_data = vec![Vec::default(); (layer_count * face_count) as usize];
    for (level, level_data) in levels.iter().enumerate() {
        let (level_width, level_height, level_depth) = (
            (width as usize >> level).max(1),
            (height as usize >> level).max(1),
            (depth as usize >> level).max(1),
        );
        let (num_blocks_x, num_blocks_y) = (
            ((level_width + block_width_pixels - 1) / block_width_pixels).max(1),
            ((level_height + block_height_pixels - 1) / block_height_pixels).max(1),
        );
        let level_bytes = num_blocks_x * num_blocks_y * level_depth * block_bytes;

        let mut index = 0;
        for _layer in 0..layer_count {
            for _face in 0..face_count {
                let offset = index * level_bytes;
                wgpu_data[index].extend_from_slice(&level_data[offset..(offset + level_bytes)]);
                index += 1;
            }
        }
    }

    // Assign the data and fill in the rest of the metadata now the possible
    // error cases have been handled
    let mut image = Image::default();
    image.texture_descriptor.format = texture_format;
    image.data = wgpu_data.into_iter().flatten().collect::<Vec<_>>();
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
pub fn get_transcoded_formats(
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
    is_srgb: bool,
) -> Result<TextureFormat, TextureError> {
    if let Some(format) = ktx2.header().format {
        return ktx2_format_to_texture_format(format, is_srgb);
    }

    for data_format_descriptor in ktx2.data_format_descriptors() {
        if data_format_descriptor.header == DataFormatDescriptorHeader::BASIC {
            let basic_data_format_descriptor =
                BasicDataFormatDescriptor::parse(data_format_descriptor.data)
                    .map_err(|err| TextureError::InvalidData(format!("KTX2: {err:?}")))?;
            let sample_information = basic_data_format_descriptor
                .sample_information()
                .collect::<Vec<_>>();
            return ktx2_dfd_to_texture_format(
                &basic_data_format_descriptor,
                &sample_information,
                is_srgb,
            );
        }
    }

    Err(TextureError::UnsupportedTextureFormat(
        "Unknown".to_string(),
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

// This can be obtained from std::mem::transmute::<f32, u32>(1.0f32). It is used for identifying
// normalized sample types as in Unorm or Snorm.
const F32_1_AS_U32: u32 = 1065353216;

fn sample_information_to_data_type(
    sample: &SampleInformation,
    is_srgb: bool,
) -> Result<DataType, TextureError> {
    // Exponent flag not supported
    if sample
        .channel_type_qualifiers
        .contains(ChannelTypeQualifiers::EXPONENT)
    {
        return Err(TextureError::UnsupportedTextureFormat(
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
    data_format_descriptor: &BasicDataFormatDescriptor,
    sample_information: &[SampleInformation],
    is_srgb: bool,
) -> Result<TextureFormat, TextureError> {
    Ok(match data_format_descriptor.color_model {
        Some(ColorModel::RGBSDA) => {
            match sample_information.len() {
                1 => {
                    // Only red channel allowed
                    if sample_information[0].channel_type != 0 {
                        return Err(TextureError::UnsupportedTextureFormat(
                            "Only red-component single-component KTX2 RGBSDA formats supported"
                                .to_string(),
                        ));
                    }

                    let sample = &sample_information[0];
                    let data_type = sample_information_to_data_type(sample, false)?;
                    match sample.bit_length {
                        8 => match data_type {
                            DataType::Unorm => TextureFormat::R8Unorm,
                            DataType::UnormSrgb => {
                                return Err(TextureError::UnsupportedTextureFormat(
                                    "UnormSrgb not supported for R8".to_string(),
                                ));
                            }
                            DataType::Snorm => TextureFormat::R8Snorm,
                            DataType::Float => {
                                return Err(TextureError::UnsupportedTextureFormat(
                                    "Float not supported for R8".to_string(),
                                ));
                            }
                            DataType::Uint => TextureFormat::R8Uint,
                            DataType::Sint => TextureFormat::R8Sint,
                        },
                        16 => match data_type {
                            DataType::Unorm => TextureFormat::R16Unorm,
                            DataType::UnormSrgb => {
                                return Err(TextureError::UnsupportedTextureFormat(
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
                                return Err(TextureError::UnsupportedTextureFormat(
                                    "Unorm not supported for R32".to_string(),
                                ));
                            }
                            DataType::UnormSrgb => {
                                return Err(TextureError::UnsupportedTextureFormat(
                                    "UnormSrgb not supported for R32".to_string(),
                                ));
                            }
                            DataType::Snorm => {
                                return Err(TextureError::UnsupportedTextureFormat(
                                    "Snorm not supported for R32".to_string(),
                                ));
                            }
                            DataType::Float => TextureFormat::R32Float,
                            DataType::Uint => TextureFormat::R32Uint,
                            DataType::Sint => TextureFormat::R32Sint,
                        },
                        v => {
                            return Err(TextureError::UnsupportedTextureFormat(format!(
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
                        return Err(TextureError::UnsupportedTextureFormat(
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
                    match sample.bit_length {
                        8 => match data_type {
                            DataType::Unorm => TextureFormat::Rg8Unorm,
                            DataType::UnormSrgb => {
                                return Err(TextureError::UnsupportedTextureFormat(
                                    "UnormSrgb not supported for Rg8".to_string(),
                                ));
                            }
                            DataType::Snorm => TextureFormat::Rg8Snorm,
                            DataType::Float => {
                                return Err(TextureError::UnsupportedTextureFormat(
                                    "Float not supported for Rg8".to_string(),
                                ));
                            }
                            DataType::Uint => TextureFormat::Rg8Uint,
                            DataType::Sint => TextureFormat::Rg8Sint,
                        },
                        16 => match data_type {
                            DataType::Unorm => TextureFormat::Rg16Unorm,
                            DataType::UnormSrgb => {
                                return Err(TextureError::UnsupportedTextureFormat(
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
                                return Err(TextureError::UnsupportedTextureFormat(
                                    "Unorm not supported for Rg32".to_string(),
                                ));
                            }
                            DataType::UnormSrgb => {
                                return Err(TextureError::UnsupportedTextureFormat(
                                    "UnormSrgb not supported for Rg32".to_string(),
                                ));
                            }
                            DataType::Snorm => {
                                return Err(TextureError::UnsupportedTextureFormat(
                                    "Snorm not supported for Rg32".to_string(),
                                ));
                            }
                            DataType::Float => TextureFormat::Rg32Float,
                            DataType::Uint => TextureFormat::Rg32Uint,
                            DataType::Sint => TextureFormat::Rg32Sint,
                        },
                        v => {
                            return Err(TextureError::UnsupportedTextureFormat(format!(
                                "Unsupported sample bit length for RGBSDA 2-channel format: {v}",
                            )));
                        }
                    }
                }
                3 => {
                    if sample_information[0].channel_type == 0
                        && sample_information[0].bit_length == 11
                        && sample_information[1].channel_type == 1
                        && sample_information[1].bit_length == 11
                        && sample_information[2].channel_type == 2
                        && sample_information[2].bit_length == 10
                    {
                        TextureFormat::Rg11b10Float
                    } else if sample_information[0].channel_type == 0
                        && sample_information[0].bit_length == 9
                        && sample_information[1].channel_type == 1
                        && sample_information[1].bit_length == 9
                        && sample_information[2].channel_type == 2
                        && sample_information[2].bit_length == 9
                    {
                        TextureFormat::Rgb9e5Ufloat
                    } else if sample_information[0].channel_type == 0
                        && sample_information[0].bit_length == 8
                        && sample_information[1].channel_type == 1
                        && sample_information[1].bit_length == 8
                        && sample_information[2].channel_type == 2
                        && sample_information[2].bit_length == 8
                    {
                        return Err(TextureError::FormatRequiresTranscodingError(
                            TranscodeFormat::Rgb8,
                        ));
                    } else {
                        return Err(TextureError::UnsupportedTextureFormat(
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
                    if sample_information[0].bit_length == 10
                        && sample_information[1].bit_length == 10
                        && sample_information[2].bit_length == 10
                        && sample_information[3].bit_length == 2
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
                    match sample.bit_length {
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
                                    return Err(TextureError::UnsupportedTextureFormat(
                                        "Bgra8 not supported for Snorm".to_string(),
                                    ));
                                }
                            }
                            DataType::Float => {
                                return Err(TextureError::UnsupportedTextureFormat(
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
                                    return Err(TextureError::UnsupportedTextureFormat(
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
                                    return Err(TextureError::UnsupportedTextureFormat(
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
                                    return Err(TextureError::UnsupportedTextureFormat(
                                        "Bgra16 not supported for Unorm".to_string(),
                                    ));
                                }
                            }
                            DataType::UnormSrgb => {
                                return Err(TextureError::UnsupportedTextureFormat(
                                    "UnormSrgb not supported for Rgba16/Bgra16".to_string(),
                                ));
                            }
                            DataType::Snorm => {
                                if is_rgba {
                                    TextureFormat::Rgba16Snorm
                                } else {
                                    return Err(TextureError::UnsupportedTextureFormat(
                                        "Bgra16 not supported for Snorm".to_string(),
                                    ));
                                }
                            }
                            DataType::Float => {
                                if is_rgba {
                                    TextureFormat::Rgba16Float
                                } else {
                                    return Err(TextureError::UnsupportedTextureFormat(
                                        "Bgra16 not supported for Float".to_string(),
                                    ));
                                }
                            }
                            DataType::Uint => {
                                if is_rgba {
                                    TextureFormat::Rgba16Uint
                                } else {
                                    return Err(TextureError::UnsupportedTextureFormat(
                                        "Bgra16 not supported for Uint".to_string(),
                                    ));
                                }
                            }
                            DataType::Sint => {
                                if is_rgba {
                                    TextureFormat::Rgba16Sint
                                } else {
                                    return Err(TextureError::UnsupportedTextureFormat(
                                        "Bgra16 not supported for Sint".to_string(),
                                    ));
                                }
                            }
                        },
                        32 => match data_type {
                            DataType::Unorm => {
                                return Err(TextureError::UnsupportedTextureFormat(
                                    "Unorm not supported for Rgba32/Bgra32".to_string(),
                                ));
                            }
                            DataType::UnormSrgb => {
                                return Err(TextureError::UnsupportedTextureFormat(
                                    "UnormSrgb not supported for Rgba32/Bgra32".to_string(),
                                ));
                            }
                            DataType::Snorm => {
                                return Err(TextureError::UnsupportedTextureFormat(
                                    "Snorm not supported for Rgba32/Bgra32".to_string(),
                                ));
                            }
                            DataType::Float => {
                                if is_rgba {
                                    TextureFormat::Rgba32Float
                                } else {
                                    return Err(TextureError::UnsupportedTextureFormat(
                                        "Bgra32 not supported for Float".to_string(),
                                    ));
                                }
                            }
                            DataType::Uint => {
                                if is_rgba {
                                    TextureFormat::Rgba32Uint
                                } else {
                                    return Err(TextureError::UnsupportedTextureFormat(
                                        "Bgra32 not supported for Uint".to_string(),
                                    ));
                                }
                            }
                            DataType::Sint => {
                                if is_rgba {
                                    TextureFormat::Rgba32Sint
                                } else {
                                    return Err(TextureError::UnsupportedTextureFormat(
                                        "Bgra32 not supported for Sint".to_string(),
                                    ));
                                }
                            }
                        },
                        v => {
                            return Err(TextureError::UnsupportedTextureFormat(format!(
                                "Unsupported sample bit length for RGBSDA 4-channel format: {v}",
                            )));
                        }
                    }
                }
                v => {
                    return Err(TextureError::UnsupportedTextureFormat(format!(
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
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                data_format_descriptor.color_model
            )));
        }
        Some(ColorModel::XYZW) => {
            // Same number of channels in both texel block dimensions and sample info descriptions
            assert_eq!(
                data_format_descriptor.texel_block_dimensions[0] as usize,
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
                    match sample.bit_length {
                        8 => match data_type {
                            DataType::Unorm => TextureFormat::Rgba8Unorm,
                            DataType::UnormSrgb => {
                                return Err(TextureError::UnsupportedTextureFormat(
                                    "UnormSrgb not supported for XYZW".to_string(),
                                ));
                            }
                            DataType::Snorm => TextureFormat::Rgba8Snorm,
                            DataType::Float => {
                                return Err(TextureError::UnsupportedTextureFormat(
                                    "Float not supported for Rgba8/Bgra8".to_string(),
                                ));
                            }
                            DataType::Uint => TextureFormat::Rgba8Uint,
                            DataType::Sint => TextureFormat::Rgba8Sint,
                        },
                        16 => match data_type {
                            DataType::Unorm => TextureFormat::Rgba16Unorm,
                            DataType::UnormSrgb => {
                                return Err(TextureError::UnsupportedTextureFormat(
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
                                return Err(TextureError::UnsupportedTextureFormat(
                                    "Unorm not supported for Rgba32/Bgra32".to_string(),
                                ));
                            }
                            DataType::UnormSrgb => {
                                return Err(TextureError::UnsupportedTextureFormat(
                                    "UnormSrgb not supported for Rgba32/Bgra32".to_string(),
                                ));
                            }
                            DataType::Snorm => {
                                return Err(TextureError::UnsupportedTextureFormat(
                                    "Snorm not supported for Rgba32/Bgra32".to_string(),
                                ));
                            }
                            DataType::Float => TextureFormat::Rgba32Float,
                            DataType::Uint => TextureFormat::Rgba32Uint,
                            DataType::Sint => TextureFormat::Rgba32Sint,
                        },
                        v => {
                            return Err(TextureError::UnsupportedTextureFormat(format!(
                                "Unsupported sample bit length for XYZW 4-channel format: {v}",
                            )));
                        }
                    }
                }
                v => {
                    return Err(TextureError::UnsupportedTextureFormat(format!(
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
                        return Err(TextureError::UnsupportedTextureFormat(format!(
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
                    return Err(TextureError::UnsupportedTextureFormat(format!(
                        "Invalid ETC2 2-sample channel types: {} {}",
                        sample0.channel_type, sample1.channel_type
                    )));
                }
            }
            v => {
                return Err(TextureError::UnsupportedTextureFormat(format!(
                    "Unsupported channel count for ETC2 format: {v}",
                )));
            }
        },
        Some(ColorModel::ASTC) => TextureFormat::Astc {
            block: match (
                data_format_descriptor.texel_block_dimensions[0],
                data_format_descriptor.texel_block_dimensions[1],
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
                    return Err(TextureError::UnsupportedTextureFormat(format!(
                        "Invalid ASTC dimension: {} x {}",
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
            return Err(TextureError::FormatRequiresTranscodingError(
                TranscodeFormat::Etc1s,
            ));
        }
        Some(ColorModel::PVRTC) => {
            return Err(TextureError::UnsupportedTextureFormat(
                "PVRTC is not supported".to_string(),
            ));
        }
        Some(ColorModel::PVRTC2) => {
            return Err(TextureError::UnsupportedTextureFormat(
                "PVRTC2 is not supported".to_string(),
            ));
        }
        Some(ColorModel::UASTC) => {
            return Err(TextureError::FormatRequiresTranscodingError(
                TranscodeFormat::Uastc(match sample_information[0].channel_type {
                    0 => DataFormat::Rgb,
                    3 => DataFormat::Rgba,
                    4 => DataFormat::Rrr,
                    5 => DataFormat::Rrrg,
                    6 => DataFormat::Rg,
                    channel_type => {
                        return Err(TextureError::UnsupportedTextureFormat(format!(
                            "Invalid KTX2 UASTC channel type: {channel_type}",
                        )))
                    }
                }),
            ));
        }
        None => {
            return Err(TextureError::UnsupportedTextureFormat(
                "Unspecified KTX2 color model".to_string(),
            ));
        }
        _ => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "Unknown KTX2 color model: {:?}",
                data_format_descriptor.color_model
            )));
        }
    })
}

pub fn ktx2_format_to_texture_format(
    ktx2_format: ktx2::Format,
    is_srgb: bool,
) -> Result<TextureFormat, TextureError> {
    Ok(match ktx2_format {
        ktx2::Format::R8_UNORM | ktx2::Format::R8_SRGB => {
            if is_srgb {
                return Err(TextureError::FormatRequiresTranscodingError(
                    TranscodeFormat::R8UnormSrgb,
                ));
            }
            TextureFormat::R8Unorm
        }
        ktx2::Format::R8_SNORM => TextureFormat::R8Snorm,
        ktx2::Format::R8_UINT => TextureFormat::R8Uint,
        ktx2::Format::R8_SINT => TextureFormat::R8Sint,
        ktx2::Format::R8G8_UNORM | ktx2::Format::R8G8_SRGB => {
            if is_srgb {
                return Err(TextureError::FormatRequiresTranscodingError(
                    TranscodeFormat::Rg8UnormSrgb,
                ));
            }
            TextureFormat::Rg8Unorm
        }
        ktx2::Format::R8G8_SNORM => TextureFormat::Rg8Snorm,
        ktx2::Format::R8G8_UINT => TextureFormat::Rg8Uint,
        ktx2::Format::R8G8_SINT => TextureFormat::Rg8Sint,
        ktx2::Format::R8G8B8_UNORM | ktx2::Format::R8G8B8_SRGB => {
            return Err(TextureError::FormatRequiresTranscodingError(
                TranscodeFormat::Rgb8,
            ));
        }
        ktx2::Format::R8G8B8A8_UNORM | ktx2::Format::R8G8B8A8_SRGB => {
            if is_srgb {
                TextureFormat::Rgba8UnormSrgb
            } else {
                TextureFormat::Rgba8Unorm
            }
        }
        ktx2::Format::R8G8B8A8_SNORM => TextureFormat::Rgba8Snorm,
        ktx2::Format::R8G8B8A8_UINT => TextureFormat::Rgba8Uint,
        ktx2::Format::R8G8B8A8_SINT => TextureFormat::Rgba8Sint,
        ktx2::Format::B8G8R8A8_UNORM | ktx2::Format::B8G8R8A8_SRGB => {
            if is_srgb {
                TextureFormat::Bgra8UnormSrgb
            } else {
                TextureFormat::Bgra8Unorm
            }
        }
        ktx2::Format::A2R10G10B10_UNORM_PACK32 => TextureFormat::Rgb10a2Unorm,

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

        ktx2::Format::R32G32B32A32_UINT => TextureFormat::Rgba32Uint,
        ktx2::Format::R32G32B32A32_SINT => TextureFormat::Rgba32Sint,
        ktx2::Format::R32G32B32A32_SFLOAT => TextureFormat::Rgba32Float,

        ktx2::Format::B10G11R11_UFLOAT_PACK32 => TextureFormat::Rg11b10Float,
        ktx2::Format::E5B9G9R9_UFLOAT_PACK32 => TextureFormat::Rgb9e5Ufloat,

        ktx2::Format::X8_D24_UNORM_PACK32 => TextureFormat::Depth24Plus,
        ktx2::Format::D32_SFLOAT => TextureFormat::Depth32Float,

        ktx2::Format::D24_UNORM_S8_UINT => TextureFormat::Depth24PlusStencil8,

        ktx2::Format::BC1_RGB_UNORM_BLOCK
        | ktx2::Format::BC1_RGB_SRGB_BLOCK
        | ktx2::Format::BC1_RGBA_UNORM_BLOCK
        | ktx2::Format::BC1_RGBA_SRGB_BLOCK => {
            if is_srgb {
                TextureFormat::Bc1RgbaUnormSrgb
            } else {
                TextureFormat::Bc1RgbaUnorm
            }
        }
        ktx2::Format::BC2_UNORM_BLOCK | ktx2::Format::BC2_SRGB_BLOCK => {
            if is_srgb {
                TextureFormat::Bc2RgbaUnormSrgb
            } else {
                TextureFormat::Bc2RgbaUnorm
            }
        }
        ktx2::Format::BC3_UNORM_BLOCK | ktx2::Format::BC3_SRGB_BLOCK => {
            if is_srgb {
                TextureFormat::Bc3RgbaUnormSrgb
            } else {
                TextureFormat::Bc3RgbaUnorm
            }
        }
        ktx2::Format::BC4_UNORM_BLOCK => TextureFormat::Bc4RUnorm,
        ktx2::Format::BC4_SNORM_BLOCK => TextureFormat::Bc4RSnorm,
        ktx2::Format::BC5_UNORM_BLOCK => TextureFormat::Bc5RgUnorm,
        ktx2::Format::BC5_SNORM_BLOCK => TextureFormat::Bc5RgSnorm,
        ktx2::Format::BC6H_UFLOAT_BLOCK => TextureFormat::Bc6hRgbUfloat,
        ktx2::Format::BC6H_SFLOAT_BLOCK => TextureFormat::Bc6hRgbFloat,
        ktx2::Format::BC7_UNORM_BLOCK | ktx2::Format::BC7_SRGB_BLOCK => {
            if is_srgb {
                TextureFormat::Bc7RgbaUnormSrgb
            } else {
                TextureFormat::Bc7RgbaUnorm
            }
        }
        ktx2::Format::ETC2_R8G8B8_UNORM_BLOCK | ktx2::Format::ETC2_R8G8B8_SRGB_BLOCK => {
            if is_srgb {
                TextureFormat::Etc2Rgb8UnormSrgb
            } else {
                TextureFormat::Etc2Rgb8Unorm
            }
        }
        ktx2::Format::ETC2_R8G8B8A1_UNORM_BLOCK | ktx2::Format::ETC2_R8G8B8A1_SRGB_BLOCK => {
            if is_srgb {
                TextureFormat::Etc2Rgb8A1UnormSrgb
            } else {
                TextureFormat::Etc2Rgb8A1Unorm
            }
        }
        ktx2::Format::ETC2_R8G8B8A8_UNORM_BLOCK | ktx2::Format::ETC2_R8G8B8A8_SRGB_BLOCK => {
            if is_srgb {
                TextureFormat::Etc2Rgba8UnormSrgb
            } else {
                TextureFormat::Etc2Rgba8Unorm
            }
        }
        ktx2::Format::EAC_R11_UNORM_BLOCK => TextureFormat::EacR11Unorm,
        ktx2::Format::EAC_R11_SNORM_BLOCK => TextureFormat::EacR11Snorm,
        ktx2::Format::EAC_R11G11_UNORM_BLOCK => TextureFormat::EacRg11Unorm,
        ktx2::Format::EAC_R11G11_SNORM_BLOCK => TextureFormat::EacRg11Snorm,
        ktx2::Format::ASTC_4x4_UNORM_BLOCK | ktx2::Format::ASTC_4x4_SRGB_BLOCK => {
            TextureFormat::Astc {
                block: AstcBlock::B4x4,
                channel: if is_srgb {
                    AstcChannel::UnormSrgb
                } else {
                    AstcChannel::Unorm
                },
            }
        }
        ktx2::Format::ASTC_5x4_UNORM_BLOCK | ktx2::Format::ASTC_5x4_SRGB_BLOCK => {
            TextureFormat::Astc {
                block: AstcBlock::B5x4,
                channel: if is_srgb {
                    AstcChannel::UnormSrgb
                } else {
                    AstcChannel::Unorm
                },
            }
        }
        ktx2::Format::ASTC_5x5_UNORM_BLOCK | ktx2::Format::ASTC_5x5_SRGB_BLOCK => {
            TextureFormat::Astc {
                block: AstcBlock::B5x5,
                channel: if is_srgb {
                    AstcChannel::UnormSrgb
                } else {
                    AstcChannel::Unorm
                },
            }
        }
        ktx2::Format::ASTC_6x5_UNORM_BLOCK | ktx2::Format::ASTC_6x5_SRGB_BLOCK => {
            TextureFormat::Astc {
                block: AstcBlock::B6x5,
                channel: if is_srgb {
                    AstcChannel::UnormSrgb
                } else {
                    AstcChannel::Unorm
                },
            }
        }
        ktx2::Format::ASTC_6x6_UNORM_BLOCK | ktx2::Format::ASTC_6x6_SRGB_BLOCK => {
            TextureFormat::Astc {
                block: AstcBlock::B6x6,
                channel: if is_srgb {
                    AstcChannel::UnormSrgb
                } else {
                    AstcChannel::Unorm
                },
            }
        }
        ktx2::Format::ASTC_8x5_UNORM_BLOCK | ktx2::Format::ASTC_8x5_SRGB_BLOCK => {
            TextureFormat::Astc {
                block: AstcBlock::B8x5,
                channel: if is_srgb {
                    AstcChannel::UnormSrgb
                } else {
                    AstcChannel::Unorm
                },
            }
        }
        ktx2::Format::ASTC_8x6_UNORM_BLOCK | ktx2::Format::ASTC_8x6_SRGB_BLOCK => {
            TextureFormat::Astc {
                block: AstcBlock::B8x6,
                channel: if is_srgb {
                    AstcChannel::UnormSrgb
                } else {
                    AstcChannel::Unorm
                },
            }
        }
        ktx2::Format::ASTC_8x8_UNORM_BLOCK | ktx2::Format::ASTC_8x8_SRGB_BLOCK => {
            TextureFormat::Astc {
                block: AstcBlock::B8x8,
                channel: if is_srgb {
                    AstcChannel::UnormSrgb
                } else {
                    AstcChannel::Unorm
                },
            }
        }
        ktx2::Format::ASTC_10x5_UNORM_BLOCK | ktx2::Format::ASTC_10x5_SRGB_BLOCK => {
            TextureFormat::Astc {
                block: AstcBlock::B10x5,
                channel: if is_srgb {
                    AstcChannel::UnormSrgb
                } else {
                    AstcChannel::Unorm
                },
            }
        }
        ktx2::Format::ASTC_10x6_UNORM_BLOCK | ktx2::Format::ASTC_10x6_SRGB_BLOCK => {
            TextureFormat::Astc {
                block: AstcBlock::B10x6,
                channel: if is_srgb {
                    AstcChannel::UnormSrgb
                } else {
                    AstcChannel::Unorm
                },
            }
        }
        ktx2::Format::ASTC_10x8_UNORM_BLOCK | ktx2::Format::ASTC_10x8_SRGB_BLOCK => {
            TextureFormat::Astc {
                block: AstcBlock::B10x8,
                channel: if is_srgb {
                    AstcChannel::UnormSrgb
                } else {
                    AstcChannel::Unorm
                },
            }
        }
        ktx2::Format::ASTC_10x10_UNORM_BLOCK | ktx2::Format::ASTC_10x10_SRGB_BLOCK => {
            TextureFormat::Astc {
                block: AstcBlock::B10x10,
                channel: if is_srgb {
                    AstcChannel::UnormSrgb
                } else {
                    AstcChannel::Unorm
                },
            }
        }
        ktx2::Format::ASTC_12x10_UNORM_BLOCK | ktx2::Format::ASTC_12x10_SRGB_BLOCK => {
            TextureFormat::Astc {
                block: AstcBlock::B12x10,
                channel: if is_srgb {
                    AstcChannel::UnormSrgb
                } else {
                    AstcChannel::Unorm
                },
            }
        }
        ktx2::Format::ASTC_12x12_UNORM_BLOCK | ktx2::Format::ASTC_12x12_SRGB_BLOCK => {
            TextureFormat::Astc {
                block: AstcBlock::B12x12,
                channel: if is_srgb {
                    AstcChannel::UnormSrgb
                } else {
                    AstcChannel::Unorm
                },
            }
        }
        _ => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{ktx2_format:?}"
            )))
        }
    })
}

#[cfg(test)]
mod tests {
    use crate::texture::CompressedImageFormats;

    use super::ktx2_buffer_to_image;

    #[test]
    fn test_ktx_levels() {
        // R8UnormSrgb textture with 4x4 pixels data and 3 levels of mipmaps
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
        let result = ktx2_buffer_to_image(&buffer, supported_compressed_formats, true);
        assert!(result.is_ok());
    }
}
