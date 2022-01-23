use std::io::Read;

use ktx2::{BasicDataFormatDescriptor, ColorModel, SampleInformation, SupercompressionScheme};
use wgpu::{Extent3d, TextureDimension, TextureFormat};

use super::{Image, TextureError};

pub fn ktx2_buffer_to_image(buffer: &[u8], is_srgb: bool) -> Result<Image, TextureError> {
    let ktx2 = ktx2::Reader::new(buffer).expect("Can't create reader");
    let ktx2_header = ktx2.header();
    let mut image = Image::default();
    if let Some(format) = ktx2_header.format {
        image.texture_descriptor.format = ktx2_format_to_texture_format(format, is_srgb)?;
        image.data = ktx2.levels().flatten().copied().collect();
    } else if let Some(supercompression_scheme) = ktx2_header.supercompression_scheme {
        match supercompression_scheme {
            SupercompressionScheme::Zstandard => {
                for (l, level) in ktx2.levels().enumerate() {
                    let mut cursor = std::io::Cursor::new(level);
                    let mut decoder = ruzstd::StreamingDecoder::new(&mut cursor)
                        .map_err(TextureError::SuperDecompressionError)?;
                    decoder.read_to_end(&mut image.data).map_err(|err| {
                        TextureError::SuperDecompressionError(format!(
                            "Failed to decompress supercompression for mip {}: {:?}",
                            l, err
                        ))
                    })?;
                }
            }
            _ => {
                return Err(TextureError::SuperDecompressionError(format!(
                    "Unsupported supercompression scheme: {:?}",
                    supercompression_scheme
                )));
            }
        }
        let data_format_descriptors = ktx2.data_format_descriptors();
        image.texture_descriptor.format =
            ktx2_dfd_to_texture_format(&data_format_descriptors[0], is_srgb)?;
    }
    image.texture_descriptor.size = Extent3d {
        width: ktx2_header.pixel_width,
        height: ktx2_header.pixel_height,
        depth_or_array_layers: if ktx2_header.layer_count > 1 {
            ktx2_header.layer_count
        } else {
            ktx2_header.pixel_depth
        },
    };
    image.texture_descriptor.mip_level_count = ktx2_header.level_count;
    image.texture_descriptor.dimension = if ktx2_header.pixel_depth > 1 {
        TextureDimension::D3
    } else if image.is_compressed() || ktx2_header.pixel_height > 1 {
        TextureDimension::D2
    } else {
        TextureDimension::D1
    };
    Ok(image)
}

pub fn ktx2_get_texture_format<Data: AsRef<[u8]>>(
    ktx2: &ktx2::Reader<Data>,
    is_srgb: bool,
) -> Result<TextureFormat, TextureError> {
    let ktx2_header = ktx2.header();
    if let Some(format) = ktx2_header.format {
        return ktx2_format_to_texture_format(format, is_srgb);
    }
    let dfds = ktx2.data_format_descriptors();
    // FIXME: How should more than one Data Format Descriptor be handled? Follow the specification.
    if let Some(dfd) = dfds.get(0) {
        return ktx2_dfd_to_texture_format(dfd, is_srgb);
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

fn sample_information_to_data_type(
    sample: &SampleInformation,
    is_srgb: bool,
) -> Result<DataType, TextureError> {
    // Exponent flag not supported
    if sample.is_exponent() {
        return Err(TextureError::UnsupportedTextureFormat(
            "Unsupported KTX2 channel type qualifier: exponent".to_string(),
        ));
    }
    Ok(if sample.is_float() {
        // If lower bound of range is 0 then unorm, else if upper bound is 1.0f32 as u32
        if sample.is_signed() {
            if sample.is_norm() {
                DataType::Snorm
            } else {
                DataType::Float
            }
        } else if is_srgb {
            DataType::UnormSrgb
        } else {
            DataType::Unorm
        }
    } else if sample.is_signed() {
        DataType::Sint
    } else {
        DataType::Uint
    })
}

pub fn ktx2_dfd_to_texture_format(
    ktx2_dfd: &BasicDataFormatDescriptor,
    is_srgb: bool,
) -> Result<TextureFormat, TextureError> {
    Ok(match ktx2_dfd.color_model {
        ColorModel::RGBSDA => {
            // Same number of channels in both texel block dimensions and sample info descriptions
            assert_eq!(
                ktx2_dfd.texel_block_dimensions[0] as usize,
                ktx2_dfd.samples.len()
            );
            match ktx2_dfd.samples.len() {
                1 => {
                    // Only red channel allowed
                    // FIXME: What about depth?
                    assert_eq!(ktx2_dfd.samples[0].channel_type, 0);

                    let sample = &ktx2_dfd.samples[0];
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
                                "Unsupported sample bit length for RGBSDA 1-channel format: {}",
                                v
                            )));
                        }
                    }
                }
                2 => {
                    // Only red and green channels allowed
                    // FIXME: What about depth stencil?
                    assert_eq!(ktx2_dfd.samples[0].channel_type, 0);
                    assert_eq!(ktx2_dfd.samples[1].channel_type, 1);
                    // Only same bit length for all channels
                    assert_eq!(
                        ktx2_dfd.samples[0].bit_length,
                        ktx2_dfd.samples[1].bit_length
                    );
                    // Only same channel type qualifiers for all channels
                    assert_eq!(
                        ktx2_dfd.samples[0].channel_type_qualifiers,
                        ktx2_dfd.samples[1].channel_type_qualifiers
                    );
                    // Only same sample range for all channels
                    assert_eq!(ktx2_dfd.samples[0].lower, ktx2_dfd.samples[1].lower);
                    assert_eq!(ktx2_dfd.samples[0].upper, ktx2_dfd.samples[1].upper);

                    let sample = &ktx2_dfd.samples[0];
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
                                "Unsupported sample bit length for RGBSDA 2-channel format: {}",
                                v
                            )));
                        }
                    }
                }
                3 => {
                    if ktx2_dfd.samples[0].channel_type == 0
                        && ktx2_dfd.samples[0].bit_length == 11
                        && ktx2_dfd.samples[1].channel_type == 1
                        && ktx2_dfd.samples[1].bit_length == 11
                        && ktx2_dfd.samples[2].channel_type == 2
                        && ktx2_dfd.samples[2].bit_length == 10
                    {
                        TextureFormat::Rg11b10Float
                    } else if ktx2_dfd.samples[0].channel_type == 0
                        && ktx2_dfd.samples[0].bit_length == 9
                        && ktx2_dfd.samples[1].channel_type == 1
                        && ktx2_dfd.samples[1].bit_length == 9
                        && ktx2_dfd.samples[2].channel_type == 2
                        && ktx2_dfd.samples[2].bit_length == 9
                    {
                        TextureFormat::Rgb9e5Ufloat
                    } else {
                        return Err(TextureError::UnsupportedTextureFormat(
                            "3-component formats not supported".to_string(),
                        ));
                    }
                }
                4 => {
                    // Only RGBA or BGRA channels allowed
                    let is_rgba = ktx2_dfd.samples[0].channel_type == 0;
                    assert!(
                        ktx2_dfd.samples[0].channel_type == 0
                            || ktx2_dfd.samples[0].channel_type == 2
                    );
                    assert_eq!(ktx2_dfd.samples[1].channel_type, 1);
                    assert_eq!(
                        ktx2_dfd.samples[2].channel_type,
                        if is_rgba { 2 } else { 0 }
                    );
                    assert_eq!(ktx2_dfd.samples[3].channel_type, 15);

                    // Handle one special packed format
                    if ktx2_dfd.samples[0].bit_length == 10
                        && ktx2_dfd.samples[1].bit_length == 10
                        && ktx2_dfd.samples[2].bit_length == 10
                        && ktx2_dfd.samples[3].bit_length == 2
                    {
                        return Ok(TextureFormat::Rgb10a2Unorm);
                    }

                    // Only same bit length for all channels
                    assert!(
                        ktx2_dfd.samples[0].bit_length == ktx2_dfd.samples[1].bit_length
                            && ktx2_dfd.samples[0].bit_length == ktx2_dfd.samples[2].bit_length
                            && ktx2_dfd.samples[0].bit_length == ktx2_dfd.samples[3].bit_length
                    );
                    // Only same channel type qualifiers for all channels
                    assert!(
                        ktx2_dfd.samples[0].channel_type_qualifiers
                            == ktx2_dfd.samples[1].channel_type_qualifiers
                            && ktx2_dfd.samples[0].channel_type_qualifiers
                                == ktx2_dfd.samples[2].channel_type_qualifiers
                            && ktx2_dfd.samples[0].channel_type_qualifiers
                                == ktx2_dfd.samples[3].channel_type_qualifiers
                    );
                    // Only same sample range for all channels
                    assert!(
                        ktx2_dfd.samples[0].lower == ktx2_dfd.samples[1].lower
                            && ktx2_dfd.samples[0].lower == ktx2_dfd.samples[2].lower
                            && ktx2_dfd.samples[0].lower == ktx2_dfd.samples[3].lower
                    );
                    assert!(
                        ktx2_dfd.samples[0].upper == ktx2_dfd.samples[1].upper
                            && ktx2_dfd.samples[0].upper == ktx2_dfd.samples[2].upper
                            && ktx2_dfd.samples[0].upper == ktx2_dfd.samples[3].upper
                    );

                    let sample = &ktx2_dfd.samples[0];
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
                                    TextureFormat::Rgba8Uint
                                } else {
                                    return Err(TextureError::UnsupportedTextureFormat(
                                        "Bgra8 not supported for Uint".to_string(),
                                    ));
                                }
                            }
                            DataType::Sint => {
                                if is_rgba {
                                    TextureFormat::Rgba8Sint
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
                                "Unsupported sample bit length for RGBSDA 4-channel format: {}",
                                v
                            )));
                        }
                    }
                }
                v => {
                    return Err(TextureError::UnsupportedTextureFormat(format!(
                        "Unsupported channel count for RGBSDA format: {}",
                        v
                    )));
                }
            }
        }
        ColorModel::YUVSDA => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_dfd.color_model
            )));
        }
        ColorModel::YIQSDA => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_dfd.color_model
            )));
        }
        ColorModel::LabSDA => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_dfd.color_model
            )));
        }
        ColorModel::CMYKA => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_dfd.color_model
            )));
        }
        ColorModel::XYZW => {
            // Same number of channels in both texel block dimensions and sample info descriptions
            assert_eq!(
                ktx2_dfd.texel_block_dimensions[0] as usize,
                ktx2_dfd.samples.len()
            );
            match ktx2_dfd.samples.len() {
                4 => {
                    // Only RGBA or BGRA channels allowed
                    assert_eq!(ktx2_dfd.samples[0].channel_type, 0);
                    assert_eq!(ktx2_dfd.samples[1].channel_type, 1);
                    assert_eq!(ktx2_dfd.samples[2].channel_type, 2);
                    assert_eq!(ktx2_dfd.samples[3].channel_type, 3);
                    // Only same bit length for all channels
                    assert!(
                        ktx2_dfd.samples[0].bit_length == ktx2_dfd.samples[1].bit_length
                            && ktx2_dfd.samples[0].bit_length == ktx2_dfd.samples[2].bit_length
                            && ktx2_dfd.samples[0].bit_length == ktx2_dfd.samples[3].bit_length
                    );
                    // Only same channel type qualifiers for all channels
                    assert!(
                        ktx2_dfd.samples[0].channel_type_qualifiers
                            == ktx2_dfd.samples[1].channel_type_qualifiers
                            && ktx2_dfd.samples[0].channel_type_qualifiers
                                == ktx2_dfd.samples[2].channel_type_qualifiers
                            && ktx2_dfd.samples[0].channel_type_qualifiers
                                == ktx2_dfd.samples[3].channel_type_qualifiers
                    );
                    // Only same sample range for all channels
                    assert!(
                        ktx2_dfd.samples[0].lower == ktx2_dfd.samples[1].lower
                            && ktx2_dfd.samples[0].lower == ktx2_dfd.samples[2].lower
                            && ktx2_dfd.samples[0].lower == ktx2_dfd.samples[3].lower
                    );
                    assert!(
                        ktx2_dfd.samples[0].upper == ktx2_dfd.samples[1].upper
                            && ktx2_dfd.samples[0].upper == ktx2_dfd.samples[2].upper
                            && ktx2_dfd.samples[0].upper == ktx2_dfd.samples[3].upper
                    );

                    let sample = &ktx2_dfd.samples[0];
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
                                "Unsupported sample bit length for XYZW 4-channel format: {}",
                                v
                            )));
                        }
                    }
                }
                v => {
                    return Err(TextureError::UnsupportedTextureFormat(format!(
                        "Unsupported channel count for XYZW format: {}",
                        v
                    )));
                }
            }
        }
        ColorModel::HSVAAng => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_dfd.color_model
            )));
        }
        ColorModel::HSLAAng => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_dfd.color_model
            )));
        }
        ColorModel::HSVAHex => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_dfd.color_model
            )));
        }
        ColorModel::HSLAHex => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_dfd.color_model
            )));
        }
        ColorModel::YCgCoA => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_dfd.color_model
            )));
        }
        ColorModel::YcCbcCrc => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_dfd.color_model
            )));
        }
        ColorModel::ICtCp => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_dfd.color_model
            )));
        }
        ColorModel::CIEXYZ => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_dfd.color_model
            )));
        }
        ColorModel::CIEXYY => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_dfd.color_model
            )));
        }
        ColorModel::BC1A => {
            if is_srgb {
                TextureFormat::Bc1RgbaUnormSrgb
            } else {
                TextureFormat::Bc1RgbaUnorm
            }
        }
        ColorModel::BC2 => {
            if is_srgb {
                TextureFormat::Bc2RgbaUnormSrgb
            } else {
                TextureFormat::Bc2RgbaUnorm
            }
        }
        ColorModel::BC3 => {
            if is_srgb {
                TextureFormat::Bc3RgbaUnormSrgb
            } else {
                TextureFormat::Bc3RgbaUnorm
            }
        }
        ColorModel::BC4 => {
            if ktx2_dfd.samples[0].lower == 0 {
                TextureFormat::Bc4RUnorm
            } else {
                TextureFormat::Bc4RSnorm
            }
        }
        // FIXME: Red and green channels can be swapped for ATI2n/3Dc
        ColorModel::BC5 => {
            if ktx2_dfd.samples[0].lower == 0 {
                TextureFormat::Bc5RgUnorm
            } else {
                TextureFormat::Bc5RgSnorm
            }
        }
        ColorModel::BC6H => {
            if ktx2_dfd.samples[0].lower == 0 {
                TextureFormat::Bc6hRgbUfloat
            } else {
                TextureFormat::Bc6hRgbSfloat
            }
        }
        ColorModel::BC7 => {
            if is_srgb {
                TextureFormat::Bc7RgbaUnormSrgb
            } else {
                TextureFormat::Bc7RgbaUnorm
            }
        }
        // FIXME: Is ETC1 a subset of ETC2?
        ColorModel::ETC1 => {
            return Err(TextureError::UnsupportedTextureFormat(
                "ETC1 is not supported".to_string(),
            ));
        }
        ColorModel::ETC2 => match ktx2_dfd.samples.len() {
            1 => {
                let sample = &ktx2_dfd.samples[0];
                match sample.channel_type {
                    0 => {
                        if ktx2_dfd.samples[0].is_signed() {
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
                let sample0 = &ktx2_dfd.samples[0];
                let sample1 = &ktx2_dfd.samples[1];
                if sample0.channel_type == 0 && sample1.channel_type == 1 {
                    if sample0.is_signed() {
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
                    "Unsupported channel count for ETC2 format: {}",
                    v
                )));
            }
        },
        ColorModel::ASTC => match ktx2_dfd.texel_block_dimensions[0] {
            4 => match ktx2_dfd.texel_block_dimensions[1] {
                4 => {
                    if is_srgb {
                        TextureFormat::Astc4x4RgbaUnormSrgb
                    } else {
                        TextureFormat::Astc4x4RgbaUnorm
                    }
                }
                d => {
                    return Err(TextureError::UnsupportedTextureFormat(format!(
                        "Invalid ASTC y-dimension: {}",
                        d
                    )))
                }
            },
            5 => match ktx2_dfd.texel_block_dimensions[1] {
                4 => {
                    if is_srgb {
                        TextureFormat::Astc5x4RgbaUnormSrgb
                    } else {
                        TextureFormat::Astc5x4RgbaUnorm
                    }
                }
                5 => {
                    if is_srgb {
                        TextureFormat::Astc5x5RgbaUnormSrgb
                    } else {
                        TextureFormat::Astc5x5RgbaUnorm
                    }
                }
                d => {
                    return Err(TextureError::UnsupportedTextureFormat(format!(
                        "Invalid ASTC y-dimension: {}",
                        d
                    )))
                }
            },
            6 => match ktx2_dfd.texel_block_dimensions[1] {
                5 => {
                    if is_srgb {
                        TextureFormat::Astc6x5RgbaUnormSrgb
                    } else {
                        TextureFormat::Astc6x5RgbaUnorm
                    }
                }
                6 => {
                    if is_srgb {
                        TextureFormat::Astc6x6RgbaUnormSrgb
                    } else {
                        TextureFormat::Astc6x6RgbaUnorm
                    }
                }
                d => {
                    return Err(TextureError::UnsupportedTextureFormat(format!(
                        "Invalid ASTC y-dimension: {}",
                        d
                    )))
                }
            },
            8 => match ktx2_dfd.texel_block_dimensions[1] {
                5 => {
                    if is_srgb {
                        TextureFormat::Astc8x5RgbaUnormSrgb
                    } else {
                        TextureFormat::Astc8x5RgbaUnorm
                    }
                }
                6 => {
                    if is_srgb {
                        TextureFormat::Astc8x6RgbaUnormSrgb
                    } else {
                        TextureFormat::Astc8x6RgbaUnorm
                    }
                }
                8 => {
                    if is_srgb {
                        TextureFormat::Astc8x8RgbaUnormSrgb
                    } else {
                        TextureFormat::Astc8x8RgbaUnorm
                    }
                }
                d => {
                    return Err(TextureError::UnsupportedTextureFormat(format!(
                        "Invalid ASTC y-dimension: {}",
                        d
                    )))
                }
            },
            10 => match ktx2_dfd.texel_block_dimensions[1] {
                5 => {
                    if is_srgb {
                        TextureFormat::Astc10x5RgbaUnormSrgb
                    } else {
                        TextureFormat::Astc10x5RgbaUnorm
                    }
                }
                6 => {
                    if is_srgb {
                        TextureFormat::Astc10x6RgbaUnormSrgb
                    } else {
                        TextureFormat::Astc10x6RgbaUnorm
                    }
                }
                8 => {
                    if is_srgb {
                        TextureFormat::Astc10x8RgbaUnormSrgb
                    } else {
                        TextureFormat::Astc10x8RgbaUnorm
                    }
                }
                10 => {
                    if is_srgb {
                        TextureFormat::Astc10x10RgbaUnormSrgb
                    } else {
                        TextureFormat::Astc10x10RgbaUnorm
                    }
                }
                d => {
                    return Err(TextureError::UnsupportedTextureFormat(format!(
                        "Invalid ASTC y-dimension: {}",
                        d
                    )))
                }
            },
            12 => match ktx2_dfd.texel_block_dimensions[1] {
                10 => {
                    if is_srgb {
                        TextureFormat::Astc12x10RgbaUnormSrgb
                    } else {
                        TextureFormat::Astc12x10RgbaUnorm
                    }
                }
                12 => {
                    if is_srgb {
                        TextureFormat::Astc12x12RgbaUnormSrgb
                    } else {
                        TextureFormat::Astc12x12RgbaUnorm
                    }
                }
                d => {
                    return Err(TextureError::UnsupportedTextureFormat(format!(
                        "Invalid ASTC y-dimension: {}",
                        d
                    )))
                }
            },
            d => {
                return Err(TextureError::UnsupportedTextureFormat(format!(
                    "Invalid ASTC x-dimension: {}",
                    d
                )))
            }
        },
        // FIXME: Needs transcoding
        ColorModel::ETC1S => {
            return Err(TextureError::UnsupportedTextureFormat(
                "ETC1S is not supported".to_string(),
            ));
        }
        ColorModel::PVRTC => {
            return Err(TextureError::UnsupportedTextureFormat(
                "PVRTC is not supported".to_string(),
            ));
        }
        ColorModel::PVRTC2 => {
            return Err(TextureError::UnsupportedTextureFormat(
                "PVRTC2 is not supported".to_string(),
            ));
        }
        // FIXME: Needs transcoding
        ColorModel::UASTC => {
            return Err(TextureError::UnsupportedTextureFormat(
                "UASTC is not supported".to_string(),
            ));
        }
        ColorModel::Unspecified => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "Unspecified KTX2 color model: {:?}",
                ktx2_dfd.color_model
            )));
        }
        _ => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "Unknown KTX2 color model: {:?}",
                ktx2_dfd.color_model
            )));
        }
    })
}

pub fn ktx2_format_to_texture_format(
    ktx2_format: ktx2::Format,
    is_srgb: bool,
) -> Result<TextureFormat, TextureError> {
    Ok(match ktx2_format {
        ktx2::Format::R4G4_UNORM_PACK8 => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
        ktx2::Format::R4G4B4A4_UNORM_PACK16 => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
        ktx2::Format::B4G4R4A4_UNORM_PACK16 => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
        ktx2::Format::R5G6B5_UNORM_PACK16 => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
        ktx2::Format::B5G6R5_UNORM_PACK16 => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
        ktx2::Format::R5G5B5A1_UNORM_PACK16 => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
        ktx2::Format::B5G5R5A1_UNORM_PACK16 => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
        ktx2::Format::A1R5G5B5_UNORM_PACK16 => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
        ktx2::Format::R8_UNORM => {
            if is_srgb {
                return Err(TextureError::UnsupportedTextureFormat(format!(
                    "{:?}",
                    ktx2_format
                )));
            } else {
                TextureFormat::R8Unorm
            }
        }
        ktx2::Format::R8_SNORM => TextureFormat::R8Snorm,
        ktx2::Format::R8_UINT => TextureFormat::R8Uint,
        ktx2::Format::R8_SINT => TextureFormat::R8Sint,
        ktx2::Format::R8_SRGB => {
            if is_srgb {
                return Err(TextureError::UnsupportedTextureFormat(format!(
                    "{:?}",
                    ktx2_format
                )));
            } else {
                TextureFormat::R8Unorm
            }
        }
        ktx2::Format::R8G8_UNORM => {
            if is_srgb {
                return Err(TextureError::UnsupportedTextureFormat(format!(
                    "{:?}",
                    ktx2_format
                )));
            } else {
                TextureFormat::Rg8Unorm
            }
        }
        ktx2::Format::R8G8_SNORM => TextureFormat::Rg8Snorm,
        ktx2::Format::R8G8_UINT => TextureFormat::Rg8Uint,
        ktx2::Format::R8G8_SINT => TextureFormat::Rg8Sint,
        ktx2::Format::R8G8_SRGB => {
            if is_srgb {
                return Err(TextureError::UnsupportedTextureFormat(format!(
                    "{:?}",
                    ktx2_format
                )));
            } else {
                TextureFormat::Rg8Unorm
            }
        }
        ktx2::Format::R8G8B8_UNORM => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
        ktx2::Format::R8G8B8_SNORM => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
        ktx2::Format::R8G8B8_UINT => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
        ktx2::Format::R8G8B8_SINT => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
        ktx2::Format::R8G8B8_SRGB => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
        ktx2::Format::B8G8R8_UNORM => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
        ktx2::Format::B8G8R8_SNORM => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
        ktx2::Format::B8G8R8_UINT => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
        ktx2::Format::B8G8R8_SINT => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
        ktx2::Format::B8G8R8_SRGB => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
        ktx2::Format::R8G8B8A8_UNORM => {
            if is_srgb {
                TextureFormat::Rgba8UnormSrgb
            } else {
                TextureFormat::Rgba8Unorm
            }
        }
        ktx2::Format::R8G8B8A8_SNORM => TextureFormat::Rgba8Snorm,
        ktx2::Format::R8G8B8A8_UINT => TextureFormat::Rgba8Uint,
        ktx2::Format::R8G8B8A8_SINT => TextureFormat::Rgba8Sint,
        ktx2::Format::R8G8B8A8_SRGB => {
            if is_srgb {
                TextureFormat::Rgba8UnormSrgb
            } else {
                TextureFormat::Rgba8Unorm
            }
        }
        ktx2::Format::B8G8R8A8_UNORM => {
            if is_srgb {
                TextureFormat::Bgra8UnormSrgb
            } else {
                TextureFormat::Bgra8Unorm
            }
        }
        ktx2::Format::B8G8R8A8_SNORM => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
        ktx2::Format::B8G8R8A8_UINT => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
        ktx2::Format::B8G8R8A8_SINT => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
        ktx2::Format::B8G8R8A8_SRGB => {
            if is_srgb {
                TextureFormat::Bgra8UnormSrgb
            } else {
                TextureFormat::Bgra8Unorm
            }
        }
        // FIXME: Is this correct?
        ktx2::Format::A2R10G10B10_UNORM_PACK32 => TextureFormat::Rgb10a2Unorm,
        ktx2::Format::A2R10G10B10_SNORM_PACK32 => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
        ktx2::Format::A2R10G10B10_UINT_PACK32 => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
        ktx2::Format::A2R10G10B10_SINT_PACK32 => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
        ktx2::Format::A2B10G10R10_UNORM_PACK32 => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
        ktx2::Format::A2B10G10R10_SNORM_PACK32 => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
        ktx2::Format::A2B10G10R10_UINT_PACK32 => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
        ktx2::Format::A2B10G10R10_SINT_PACK32 => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
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
        ktx2::Format::R16G16B16_UNORM => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
        ktx2::Format::R16G16B16_SNORM => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
        ktx2::Format::R16G16B16_UINT => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
        ktx2::Format::R16G16B16_SINT => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
        ktx2::Format::R16G16B16_SFLOAT => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
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
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
        ktx2::Format::R32G32B32_SINT => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
        ktx2::Format::R32G32B32_SFLOAT => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
        ktx2::Format::R32G32B32A32_UINT => TextureFormat::Rgba32Uint,
        ktx2::Format::R32G32B32A32_SINT => TextureFormat::Rgba32Sint,
        ktx2::Format::R32G32B32A32_SFLOAT => TextureFormat::Rgba32Float,
        ktx2::Format::R64_UINT => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
        ktx2::Format::R64_SINT => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
        ktx2::Format::R64_SFLOAT => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
        ktx2::Format::R64G64_UINT => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
        ktx2::Format::R64G64_SINT => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
        ktx2::Format::R64G64_SFLOAT => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
        ktx2::Format::R64G64B64_UINT => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
        ktx2::Format::R64G64B64_SINT => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
        ktx2::Format::R64G64B64_SFLOAT => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
        ktx2::Format::R64G64B64A64_UINT => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
        ktx2::Format::R64G64B64A64_SINT => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
        ktx2::Format::R64G64B64A64_SFLOAT => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
        // FIXME: Is this correct?
        ktx2::Format::B10G11R11_UFLOAT_PACK32 => TextureFormat::Rg11b10Float,
        // FIXME: Is this correct?
        ktx2::Format::E5B9G9R9_UFLOAT_PACK32 => TextureFormat::Rgb9e5Ufloat,
        ktx2::Format::D16_UNORM => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
        ktx2::Format::X8_D24_UNORM_PACK32 => TextureFormat::Depth24Plus,
        ktx2::Format::D32_SFLOAT => TextureFormat::Depth32Float,
        ktx2::Format::S8_UINT => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
        ktx2::Format::D16_UNORM_S8_UINT => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
        ktx2::Format::D24_UNORM_S8_UINT => TextureFormat::Depth24PlusStencil8,
        ktx2::Format::D32_SFLOAT_S8_UINT => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
        ktx2::Format::BC1_RGB_UNORM_BLOCK => {
            if is_srgb {
                TextureFormat::Bc1RgbaUnormSrgb
            } else {
                TextureFormat::Bc1RgbaUnorm
            }
        }
        ktx2::Format::BC1_RGB_SRGB_BLOCK => {
            if is_srgb {
                TextureFormat::Bc1RgbaUnormSrgb
            } else {
                TextureFormat::Bc1RgbaUnorm
            }
        }
        ktx2::Format::BC1_RGBA_UNORM_BLOCK => {
            if is_srgb {
                TextureFormat::Bc1RgbaUnormSrgb
            } else {
                TextureFormat::Bc1RgbaUnorm
            }
        }
        ktx2::Format::BC1_RGBA_SRGB_BLOCK => {
            if is_srgb {
                TextureFormat::Bc1RgbaUnormSrgb
            } else {
                TextureFormat::Bc1RgbaUnorm
            }
        }
        ktx2::Format::BC2_UNORM_BLOCK => {
            if is_srgb {
                TextureFormat::Bc2RgbaUnormSrgb
            } else {
                TextureFormat::Bc2RgbaUnorm
            }
        }
        ktx2::Format::BC2_SRGB_BLOCK => {
            if is_srgb {
                TextureFormat::Bc2RgbaUnormSrgb
            } else {
                TextureFormat::Bc2RgbaUnorm
            }
        }
        ktx2::Format::BC3_UNORM_BLOCK => {
            if is_srgb {
                TextureFormat::Bc3RgbaUnormSrgb
            } else {
                TextureFormat::Bc3RgbaUnorm
            }
        }
        ktx2::Format::BC3_SRGB_BLOCK => {
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
        ktx2::Format::BC6H_SFLOAT_BLOCK => TextureFormat::Bc6hRgbSfloat,
        ktx2::Format::BC7_UNORM_BLOCK => {
            if is_srgb {
                TextureFormat::Bc7RgbaUnormSrgb
            } else {
                TextureFormat::Bc7RgbaUnorm
            }
        }
        ktx2::Format::BC7_SRGB_BLOCK => {
            if is_srgb {
                TextureFormat::Bc7RgbaUnormSrgb
            } else {
                TextureFormat::Bc7RgbaUnorm
            }
        }
        ktx2::Format::ETC2_R8G8B8_UNORM_BLOCK => {
            if is_srgb {
                TextureFormat::Etc2Rgb8UnormSrgb
            } else {
                TextureFormat::Etc2Rgb8Unorm
            }
        }
        ktx2::Format::ETC2_R8G8B8_SRGB_BLOCK => {
            if is_srgb {
                TextureFormat::Etc2Rgb8UnormSrgb
            } else {
                TextureFormat::Etc2Rgb8Unorm
            }
        }
        ktx2::Format::ETC2_R8G8B8A1_UNORM_BLOCK => {
            if is_srgb {
                TextureFormat::Etc2Rgb8A1UnormSrgb
            } else {
                TextureFormat::Etc2Rgb8A1Unorm
            }
        }
        ktx2::Format::ETC2_R8G8B8A1_SRGB_BLOCK => {
            if is_srgb {
                TextureFormat::Etc2Rgb8A1UnormSrgb
            } else {
                TextureFormat::Etc2Rgb8A1Unorm
            }
        }
        ktx2::Format::ETC2_R8G8B8A8_UNORM_BLOCK => {
            if is_srgb {
                TextureFormat::Etc2Rgba8UnormSrgb
            } else {
                TextureFormat::Etc2Rgba8Unorm
            }
        }
        ktx2::Format::ETC2_R8G8B8A8_SRGB_BLOCK => {
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
        ktx2::Format::ASTC_4x4_UNORM_BLOCK => {
            if is_srgb {
                TextureFormat::Astc4x4RgbaUnormSrgb
            } else {
                TextureFormat::Astc4x4RgbaUnorm
            }
        }
        ktx2::Format::ASTC_4x4_SRGB_BLOCK => {
            if is_srgb {
                TextureFormat::Astc4x4RgbaUnormSrgb
            } else {
                TextureFormat::Astc4x4RgbaUnorm
            }
        }
        ktx2::Format::ASTC_5x4_UNORM_BLOCK => {
            if is_srgb {
                TextureFormat::Astc5x4RgbaUnormSrgb
            } else {
                TextureFormat::Astc5x4RgbaUnorm
            }
        }
        ktx2::Format::ASTC_5x4_SRGB_BLOCK => {
            if is_srgb {
                TextureFormat::Astc5x4RgbaUnormSrgb
            } else {
                TextureFormat::Astc5x4RgbaUnorm
            }
        }
        ktx2::Format::ASTC_5x5_UNORM_BLOCK => {
            if is_srgb {
                TextureFormat::Astc5x5RgbaUnormSrgb
            } else {
                TextureFormat::Astc5x5RgbaUnorm
            }
        }
        ktx2::Format::ASTC_5x5_SRGB_BLOCK => {
            if is_srgb {
                TextureFormat::Astc5x5RgbaUnormSrgb
            } else {
                TextureFormat::Astc5x5RgbaUnorm
            }
        }
        ktx2::Format::ASTC_6x5_UNORM_BLOCK => {
            if is_srgb {
                TextureFormat::Astc6x5RgbaUnormSrgb
            } else {
                TextureFormat::Astc6x5RgbaUnorm
            }
        }
        ktx2::Format::ASTC_6x5_SRGB_BLOCK => {
            if is_srgb {
                TextureFormat::Astc6x5RgbaUnormSrgb
            } else {
                TextureFormat::Astc6x5RgbaUnorm
            }
        }
        ktx2::Format::ASTC_6x6_UNORM_BLOCK => {
            if is_srgb {
                TextureFormat::Astc6x6RgbaUnormSrgb
            } else {
                TextureFormat::Astc6x6RgbaUnorm
            }
        }
        ktx2::Format::ASTC_6x6_SRGB_BLOCK => {
            if is_srgb {
                TextureFormat::Astc6x6RgbaUnormSrgb
            } else {
                TextureFormat::Astc6x6RgbaUnorm
            }
        }
        ktx2::Format::ASTC_8x5_UNORM_BLOCK => {
            if is_srgb {
                TextureFormat::Astc8x5RgbaUnormSrgb
            } else {
                TextureFormat::Astc8x5RgbaUnorm
            }
        }
        ktx2::Format::ASTC_8x5_SRGB_BLOCK => {
            if is_srgb {
                TextureFormat::Astc8x5RgbaUnormSrgb
            } else {
                TextureFormat::Astc8x5RgbaUnorm
            }
        }
        ktx2::Format::ASTC_8x6_UNORM_BLOCK => {
            if is_srgb {
                TextureFormat::Astc8x6RgbaUnormSrgb
            } else {
                TextureFormat::Astc8x6RgbaUnorm
            }
        }
        ktx2::Format::ASTC_8x6_SRGB_BLOCK => {
            if is_srgb {
                TextureFormat::Astc8x6RgbaUnormSrgb
            } else {
                TextureFormat::Astc8x6RgbaUnorm
            }
        }
        ktx2::Format::ASTC_8x8_UNORM_BLOCK => {
            if is_srgb {
                TextureFormat::Astc8x8RgbaUnormSrgb
            } else {
                TextureFormat::Astc8x8RgbaUnorm
            }
        }
        ktx2::Format::ASTC_8x8_SRGB_BLOCK => {
            if is_srgb {
                TextureFormat::Astc8x8RgbaUnormSrgb
            } else {
                TextureFormat::Astc8x8RgbaUnorm
            }
        }
        ktx2::Format::ASTC_10x5_UNORM_BLOCK => {
            if is_srgb {
                TextureFormat::Astc10x5RgbaUnormSrgb
            } else {
                TextureFormat::Astc10x5RgbaUnorm
            }
        }
        ktx2::Format::ASTC_10x5_SRGB_BLOCK => {
            if is_srgb {
                TextureFormat::Astc10x5RgbaUnormSrgb
            } else {
                TextureFormat::Astc10x5RgbaUnorm
            }
        }
        ktx2::Format::ASTC_10x6_UNORM_BLOCK => {
            if is_srgb {
                TextureFormat::Astc10x6RgbaUnormSrgb
            } else {
                TextureFormat::Astc10x6RgbaUnorm
            }
        }
        ktx2::Format::ASTC_10x6_SRGB_BLOCK => {
            if is_srgb {
                TextureFormat::Astc10x6RgbaUnormSrgb
            } else {
                TextureFormat::Astc10x6RgbaUnorm
            }
        }
        ktx2::Format::ASTC_10x8_UNORM_BLOCK => {
            if is_srgb {
                TextureFormat::Astc10x8RgbaUnormSrgb
            } else {
                TextureFormat::Astc10x8RgbaUnorm
            }
        }
        ktx2::Format::ASTC_10x8_SRGB_BLOCK => {
            if is_srgb {
                TextureFormat::Astc10x8RgbaUnormSrgb
            } else {
                TextureFormat::Astc10x8RgbaUnorm
            }
        }
        ktx2::Format::ASTC_10x10_UNORM_BLOCK => {
            if is_srgb {
                TextureFormat::Astc10x10RgbaUnormSrgb
            } else {
                TextureFormat::Astc10x10RgbaUnorm
            }
        }
        ktx2::Format::ASTC_10x10_SRGB_BLOCK => {
            if is_srgb {
                TextureFormat::Astc10x10RgbaUnormSrgb
            } else {
                TextureFormat::Astc10x10RgbaUnorm
            }
        }
        ktx2::Format::ASTC_12x10_UNORM_BLOCK => {
            if is_srgb {
                TextureFormat::Astc12x10RgbaUnormSrgb
            } else {
                TextureFormat::Astc12x10RgbaUnorm
            }
        }
        ktx2::Format::ASTC_12x10_SRGB_BLOCK => {
            if is_srgb {
                TextureFormat::Astc12x10RgbaUnormSrgb
            } else {
                TextureFormat::Astc12x10RgbaUnorm
            }
        }
        ktx2::Format::ASTC_12x12_UNORM_BLOCK => {
            if is_srgb {
                TextureFormat::Astc12x12RgbaUnormSrgb
            } else {
                TextureFormat::Astc12x12RgbaUnorm
            }
        }
        ktx2::Format::ASTC_12x12_SRGB_BLOCK => {
            if is_srgb {
                TextureFormat::Astc12x12RgbaUnormSrgb
            } else {
                TextureFormat::Astc12x12RgbaUnorm
            }
        }
        _ => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{:?}",
                ktx2_format
            )))
        }
    })
}
