use super::{CompressedImageFormats, Image, TextureError};
use basis_universal::{
    BasisTextureType, DecodeFlags, Ktx2TranscodeParameters, Ktx2Transcoder, TranscoderTextureFormat,
};
use wgpu_types::{AstcBlock, AstcChannel, Extent3d, TextureDimension, TextureFormat};

/// Decodes/transcodes a KTX2 image using `basis-universal-sys`.
///
/// Note: This implementation only works for ETC1S and UASTC LDR/HDR formats. Non-UASTC ASTC textures are not supported.
pub fn ktx2_buffer_to_image_using_basisu(
    buffer: &[u8],
    supported_compressed_formats: CompressedImageFormats,
) -> Result<Image, TextureError> {
    let mut transcoder = Ktx2Transcoder::new(buffer).map_err(|_| {
        TextureError::TranscodeError(
            "Failed to open KTX2 using basis-universal. Check that it is a valid ETC1S or UASTC LDR/HDR texture (non-UASTC ASTC textures are not supported).".to_string()
        )
    })?;

    let Some(image0_info) = transcoder.image_level_info(0, 0, 0) else {
        return Err(TextureError::InvalidData(
            "Failed to get image info".to_string(),
        ));
    };

    let source_is_srgb = transcoder.is_srgb();
    let source_is_hdr = transcoder.is_hdr();
    let source_block_size = transcoder.block_size();

    // First deal with transcoding to the desired format
    // FIXME: Use external metadata to transcode to more appropriate formats for 1- or 2-component sources
    let (transcode_format, texture_format) = get_transcoded_formats(
        supported_compressed_formats,
        source_is_srgb,
        source_is_hdr,
        source_block_size,
    );

    let basis_texture_format = transcoder.basis_texture_format();
    if !basis_texture_format.can_transcode_to_format(transcode_format) {
        return Err(TextureError::UnsupportedTextureFormat(format!(
            "{basis_texture_format:?} cannot be transcoded to {transcode_format:?}",
        )));
    }
    transcoder.prepare_transcoding().map_err(|_| {
        TextureError::TranscodeError(format!(
            "Failed to prepare for transcoding from {basis_texture_format:?}",
        ))
    })?;
    let mut transcoded = Vec::new();

    let layer_count = transcoder.layer_count().max(1);
    let level_count = transcoder.level_count().max(1);
    let face_count = transcoder.face_count().max(1);

    // https://github.khronos.org/KTX-Specification/ktxspec.v2.html#_texture_type
    let texture_type = transcoder.texture_type().map_err(|err| {
        TextureError::UnsupportedTextureFormat(format!("Unsupported texture type: {:?}", err))
    })?;

    for layer_index in 0..layer_count {
        if let Some(image_info) = transcoder.image_level_info(0, layer_index, 0) {
            if image_info.orig_width != image0_info.orig_width
                || image_info.orig_height != image0_info.orig_height
            {
                return Err(TextureError::UnsupportedTextureFormat(format!(
                    "KTX2 file with multiple 2D textures with different sizes not supported. Layer {} {}x{} (expected {}x{})",
                    layer_index,
                    image_info.orig_width,
                    image_info.orig_height,
                    image0_info.orig_width,
                    image0_info.orig_height,
                )));
            }
        }
        for face_index in 0..face_count {
            for level_index in 0..level_count {
                let mut data = transcoder
                .transcode_image_level(
                    transcode_format,
                    Ktx2TranscodeParameters {
                        layer_index,
                        level_index,
                        face_index,
                        decode_flags: Some(DecodeFlags::HIGH_QUALITY),
                        ..Default::default()
                    },
                )
                .map_err(|error| {
                    TextureError::TranscodeError(format!(
                        "Failed to transcode mip level {level_index} (face {face_index}) from {basis_texture_format:?} to {transcode_format:?}: {error:?}",
                    ))
                })?;
                transcoded.append(&mut data);
            }
        }
    }

    // Then prepare the Image
    let mut image = Image::default();
    image.texture_descriptor.size = Extent3d {
        width: image0_info.orig_width,
        height: image0_info.orig_height,
        depth_or_array_layers: layer_count,
    }
    .physical_size(texture_format);
    image.texture_descriptor.mip_level_count = level_count;
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
    image.data = Some(transcoded);
    Ok(image)
}

pub fn get_transcoded_formats(
    supported_compressed_formats: CompressedImageFormats,
    source_is_srgb: bool,
    source_is_hdr: bool,
    source_block_size: (u32, u32),
) -> (TranscoderTextureFormat, TextureFormat) {
    // NOTE: UASTC can be losslessly transcoded to ASTC4x4 and ASTC uses the same
    // space as BC7 (128-bits per 4x4 texel block) so prefer ASTC over BC for
    // transcoding speed and quality.
    if source_is_hdr && supported_compressed_formats.contains(CompressedImageFormats::ASTC_HDR) {
        match source_block_size {
            (6, 6) => (
                TranscoderTextureFormat::ASTC_HDR_6x6_RGBA,
                TextureFormat::Astc {
                    block: AstcBlock::B6x6,
                    channel: AstcChannel::Hdr,
                },
            ),
            _ => (
                TranscoderTextureFormat::ASTC_HDR_4x4_RGBA,
                TextureFormat::Astc {
                    block: AstcBlock::B4x4,
                    channel: AstcChannel::Hdr,
                },
            ),
        }
    } else if supported_compressed_formats.contains(CompressedImageFormats::ASTC_LDR) {
        (
            TranscoderTextureFormat::ASTC_4x4_RGBA,
            TextureFormat::Astc {
                block: AstcBlock::B4x4,
                channel: if source_is_srgb {
                    AstcChannel::UnormSrgb
                } else {
                    AstcChannel::Unorm
                },
            },
        )
    } else if supported_compressed_formats.contains(CompressedImageFormats::BC) {
        (
            TranscoderTextureFormat::BC7_RGBA,
            if source_is_srgb {
                TextureFormat::Bc7RgbaUnormSrgb
            } else {
                TextureFormat::Bc7RgbaUnorm
            },
        )
    } else if supported_compressed_formats.contains(CompressedImageFormats::ETC2) {
        (
            TranscoderTextureFormat::ETC2_RGBA,
            if source_is_srgb {
                TextureFormat::Etc2Rgba8UnormSrgb
            } else {
                TextureFormat::Etc2Rgba8Unorm
            },
        )
    } else {
        (
            TranscoderTextureFormat::RGBA32,
            if source_is_srgb {
                TextureFormat::Rgba8UnormSrgb
            } else {
                TextureFormat::Rgba8Unorm
            },
        )
    }
}
