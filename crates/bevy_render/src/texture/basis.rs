use basis_universal::{
    BasisTextureType, DecodeFlags, TranscodeParameters, Transcoder, TranscoderTextureFormat,
};
use wgpu::{AstcBlock, AstcChannel, Extent3d, TextureDimension, TextureFormat};

use super::{CompressedImageFormats, Image, TextureError};

pub fn basis_buffer_to_image(
    buffer: &[u8],
    supported_compressed_formats: CompressedImageFormats,
    is_srgb: bool,
) -> Result<Image, TextureError> {
    let mut transcoder = Transcoder::new();

    #[cfg(debug_assertions)]
    if !transcoder.validate_file_checksums(buffer, true) {
        return Err(TextureError::InvalidData("Invalid checksum".to_string()));
    }
    if !transcoder.validate_header(buffer) {
        return Err(TextureError::InvalidData("Invalid header".to_string()));
    }

    let Some(image0_info) = transcoder.image_info(buffer, 0) else {
        return Err(TextureError::InvalidData(
            "Failed to get image info".to_string(),
        ));
    };

    // First deal with transcoding to the desired format
    // FIXME: Use external metadata to transcode to more appropriate formats for 1- or 2-component sources
    let (transcode_format, texture_format) =
        get_transcoded_formats(supported_compressed_formats, is_srgb);
    let basis_texture_format = transcoder.basis_texture_format(buffer);
    if !basis_texture_format.can_transcode_to_format(transcode_format) {
        return Err(TextureError::UnsupportedTextureFormat(format!(
            "{basis_texture_format:?} cannot be transcoded to {transcode_format:?}",
        )));
    }
    transcoder.prepare_transcoding(buffer).map_err(|_| {
        TextureError::TranscodeError(format!(
            "Failed to prepare for transcoding from {basis_texture_format:?}",
        ))
    })?;
    let mut transcoded = Vec::new();

    let image_count = transcoder.image_count(buffer);
    let texture_type = transcoder.basis_texture_type(buffer);
    if texture_type == BasisTextureType::TextureTypeCubemapArray && image_count % 6 != 0 {
        return Err(TextureError::InvalidData(format!(
            "Basis file with cube map array texture with non-modulo 6 number of images: {image_count}",
        )));
    }

    let image0_mip_level_count = transcoder.image_level_count(buffer, 0);
    for image_index in 0..image_count {
        if let Some(image_info) = transcoder.image_info(buffer, image_index) {
            if texture_type == BasisTextureType::TextureType2D
                && (image_info.m_orig_width != image0_info.m_orig_width
                    || image_info.m_orig_height != image0_info.m_orig_height)
            {
                return Err(TextureError::UnsupportedTextureFormat(format!(
                    "Basis file with multiple 2D textures with different sizes not supported. Image {} {}x{}, image 0 {}x{}",
                    image_index,
                    image_info.m_orig_width,
                    image_info.m_orig_height,
                    image0_info.m_orig_width,
                    image0_info.m_orig_height,
                )));
            }
        }
        let mip_level_count = transcoder.image_level_count(buffer, image_index);
        if mip_level_count != image0_mip_level_count {
            return Err(TextureError::InvalidData(format!(
                "Array or volume texture has inconsistent number of mip levels. Image {image_index} has {mip_level_count} but image 0 has {image0_mip_level_count}",
            )));
        }
        for level_index in 0..mip_level_count {
            let mut data = transcoder
                .transcode_image_level(
                    buffer,
                    transcode_format,
                    TranscodeParameters {
                        image_index,
                        level_index,
                        decode_flags: Some(DecodeFlags::HIGH_QUALITY),
                        ..Default::default()
                    },
                )
                .map_err(|error| {
                    TextureError::TranscodeError(format!(
                        "Failed to transcode mip level {level_index} from {basis_texture_format:?} to {transcode_format:?}: {error:?}",
                    ))
                })?;
            transcoded.append(&mut data);
        }
    }

    // Then prepare the Image
    let mut image = Image::default();
    image.texture_descriptor.size = Extent3d {
        width: image0_info.m_orig_width,
        height: image0_info.m_orig_height,
        depth_or_array_layers: image_count,
    }
    .physical_size(texture_format);
    image.texture_descriptor.mip_level_count = image0_mip_level_count;
    image.texture_descriptor.format = texture_format;
    image.texture_descriptor.dimension = match texture_type {
        BasisTextureType::TextureType2D
        | BasisTextureType::TextureType2DArray
        | BasisTextureType::TextureTypeCubemapArray => TextureDimension::D2,
        BasisTextureType::TextureTypeVolume => TextureDimension::D3,
        basis_texture_type => {
            return Err(TextureError::UnsupportedTextureFormat(format!(
                "{basis_texture_type:?}",
            )))
        }
    };
    image.data = transcoded;
    Ok(image)
}

pub fn get_transcoded_formats(
    supported_compressed_formats: CompressedImageFormats,
    is_srgb: bool,
) -> (TranscoderTextureFormat, TextureFormat) {
    // NOTE: UASTC can be losslessly transcoded to ASTC4x4 and ASTC uses the same
    // space as BC7 (128-bits per 4x4 texel block) so prefer ASTC over BC for
    // transcoding speed and quality.
    if supported_compressed_formats.contains(CompressedImageFormats::ASTC_LDR) {
        (
            TranscoderTextureFormat::ASTC_4x4_RGBA,
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
            TranscoderTextureFormat::BC7_RGBA,
            if is_srgb {
                TextureFormat::Bc7RgbaUnormSrgb
            } else {
                TextureFormat::Bc7RgbaUnorm
            },
        )
    } else if supported_compressed_formats.contains(CompressedImageFormats::ETC2) {
        (
            TranscoderTextureFormat::ETC2_RGBA,
            if is_srgb {
                TextureFormat::Etc2Rgba8UnormSrgb
            } else {
                TextureFormat::Etc2Rgba8Unorm
            },
        )
    } else {
        (
            TranscoderTextureFormat::RGBA32,
            if is_srgb {
                TextureFormat::Rgba8UnormSrgb
            } else {
                TextureFormat::Rgba8Unorm
            },
        )
    }
}
